use self::round_robin::RoundRobin;
use crate::task::{self, Identifier, Task};
use alloc::sync::Arc;
use core::cell::RefCell;
use macros::{init, per_cpu};
use sync::Lazy;
use x86_64::thread;

pub mod round_robin;

/// The scheduler used by the kernel. This is a global variable to allow changing the scheduler
/// implementation at compile time more easily.
static SCHEDULER: Lazy<RoundRobin> = Lazy::new(RoundRobin::default);

/// When a task is terminated, we cannot drop it immediately because it will be still used until
/// the scheduler has scheduled another task. Unfortunately, the scheduling is done in assembly,
/// meaning that we cannot drop the task here.
/// To solve this problem, we store the task to drop here, and drop it after the next scheduler
/// call.
#[per_cpu]
static DROP_LATER: RefCell<Option<Arc<Task>>> = RefCell::new(None);

/// A trait that represents a scheduler. This trait is used to abstract the scheduler
/// implementation, allowing us to easily switch between different schedulers.
pub trait Scheduler {
    /// Return the current task running on the CPU. This function should return `None` if no task
    /// is currently running on the CPU (meaning that the CPU is idle)
    fn current_task(&self) -> Option<Arc<Task>>;

    /// Set the current task running on the CPU, and set the task state to `Running`.
    fn set_current_task(&self, task: Arc<Task>);

    /// Pick the next thread to run if no thread is available, this function should wait until a
    /// thread is available.
    /// Note that this function can return the current thread, and this case should be correctly
    /// handled by the caller.
    fn pick_next(&self) -> Arc<Task>;

    /// Add a task to the scheduler. The task will be added to the run queue, and will be
    /// available to be run by the scheduler.
    fn add_task(&self, task: Arc<Task>);

    /// Remove a thread from the scheduler. The thread will be removed from the run queue, and
    /// cannot be run until it is added again. If the task is currently running, this function
    /// removes it from the run queue, but does not stop it, only preventing it from being
    /// rexecuted when it yields.
    fn remove_task(&self, tid: task::Identifier);

    /// Return a task from its identifier if it exists, or `None` otherwise.
    fn task(&self, tid: task::Identifier) -> Option<Arc<Task>>;

    /// This function is called every time a timer tick occurs. It is used to update thread
    /// scheduling information, and eventually to reschedule the current thread.
    fn timer_tick(&self);

    /// Schedule the current thread.
    ///
    /// # Safety
    unsafe fn schedule(&self) {
        let next = self.pick_next();
        let current = self.current_task().unwrap();

        self.set_current_task(Arc::clone(&next));

        // If the previous thread was terminated, we need to drop it here. For more information,
        // see the comment on the `DROP_LATER` static variable.
        DROP_LATER.local().borrow_mut().take();

        // If the next thread is the same as the current one,
        // we do not need to switch threads (obviously)
        if current.id() != next.id() {
            // Here, we force the lock of the next thread. This is necessary because the switching
            // code is written in assembly, and we cannot unlock the thread lock in assembly when 
            // we have saved the state of this thread. Therefore, we unlock the thread lock here,
            // but we must forget the lock guard further down, otherwise the lock will be unlocked
            // twice.
            next.thread().force_unlock();

            let mut next = next.thread().lock();
            match current.state() {
                // If the current task is terminated, we need drop the thread later, and we
                // does not need to save its state, since it will never be scheduled again.
                task::State::Terminated => {
                    DROP_LATER.local().borrow_mut().replace(current);
                    x86_64::thread::jump_to_thread(&mut next)
                }

                // If the current task is still in the running state, we need to change its
                // state to `Ready` before switching to the next task.
                task::State::Running => current.change_state(task::State::Ready),

                // If the current task is blocked, we do not need to do anything, not even
                // change its state, because it will be automatically changed when it will
                // be unblocked.
                task::State::Blocked => (),

                // Other states are not supposed to be scheduled and it is a bug if we are
                // here. We panic in this case, because this is a bug in the kernel that
                // we cannot recover from and should be fixed.
                _ => unreachable!(),
            }

            // Lock the current thread to allow switching saving its state
            let mut prev = current.thread().lock();
            x86_64::thread::switch(&mut prev, &mut next);

            // If we are here, that means that we have been rescheduled and our thread
            // has been switched back to. We can now safely unlock the thread lock.
            //
            // We must forget the lock guard, because the lock was previously unlocked when
            // reswitching to this thread. If we do not forget the lock guard, the lock will
            // be unlocked twice, which is undefined behavior and will most likely cause a
            // panic.
            core::mem::forget(next);
        }
    }
}

/// Setup the scheduler
#[init]
pub fn setup() {
    Lazy::force(&SCHEDULER);
}

/// Run the init task. This function is only supposed to be called once, after the kernel startup
/// in order to run the init task.
///
/// # Safety
/// This function is unsafe because it make some assumptions about the state of the system
/// (e.g. that the scheduler has been initialized, that the init task exists, etc.). If these
/// assumptions are not met, this function will panic. Furthermore,
pub unsafe fn run_init() {
    let init = task(Identifier::new(1)).expect("Init task not found");
    SCHEDULER.set_current_task(Arc::clone(&init));
    thread::jump_to_thread(&mut init.thread().lock());
}

/// Add a task to the scheduler. The task will be added to the run queue, and will be
/// available to be run by the scheduler.
pub fn add_task(task: Arc<Task>) {
    SCHEDULER.add_task(task);
}

/// Remove a task from the scheduler. The task will be removed from the run queue, and
/// cannot be run until it is added again. If the task is currently running, this function
/// removes it from the run queue, but does not stop it, only preventing it from being
/// rexecuted when it yields.
pub fn remove_task(tid: task::Identifier) {
    SCHEDULER.remove_task(tid);
}

/// Called every time a timer tick occurs. It is used to update thread scheduling
/// information, and eventually to reschedule the current thread.
pub fn timer_tick() {
    SCHEDULER.timer_tick();
}

/// Schedule the current thread.
///
/// # Safety
/// This function is unsafe because it performs a context switch, and thus must be called
/// with care, as it may break code, cause memory corruption, deadlocks... if not used
/// correctly.
pub unsafe fn schedule() {
    {
        // If the current thread state is `Running`, we change it to `Ready`. We do
        // this here because the schedule function assume that the current thread
        // state is not `Running`. This also allow the scheduler the continue this
        // thread immediately if it is the only one available.
        let current = current_task().unwrap();
        if let task::State::Running = current.state() {
            current.change_state(task::State::Ready)
        }
    }
    SCHEDULER.schedule();
}

/// Return the current task running on the CPU. This function should return `None` if no task
/// is currently running on the CPU (meaning that the CPU is idle)
pub fn current_task() -> Option<Arc<Task>> {
    SCHEDULER.current_task()
}

/// Return a task from its identifier if it exists, or `None` otherwise.
pub fn task(tid: task::Identifier) -> Option<Arc<Task>> {
    SCHEDULER.task(tid)
}
