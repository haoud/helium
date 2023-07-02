use self::round_robin::RoundRobin;
use super::task::{self, Task};
use crate::x86_64;
use alloc::sync::Arc;
use macros::init;
use sync::Lazy;

pub mod round_robin;

/// The scheduler used by the kernel. This is a global variable to allow changing the scheduler
/// implementation at compile time more easily.
static SCHEDULER: Lazy<RoundRobin> = Lazy::new(RoundRobin::new);

/// A trait that represents a scheduler. This trait is used to abstract the scheduler
/// implementation, allowing us to easily switch between different schedulers.
pub trait Scheduler {
    /// Return the current task running on the CPU.
    fn current_task(&self) -> Arc<Task>;

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

    /// Run the AP. This function is called when the scheduler detect that there is no current
    /// thread running on the CPU. It that context, it can only mean that the CPU is an AP that
    /// has just been booted, and that it needs to be run a process.
    /// So, this function should pick a thread to run (and wait if no thread is available), and
    /// run it.
    ///
    /// # Safety
    unsafe fn engage_cpu(&self) -> ! {
        let next = self.pick_next();
        self.set_current_task(Arc::clone(&next));

        // Here, we manually decrement the strong count of the next task. This is needed
        // because when we switch to the next task, we will never return from the jump_to
        // function, and the strong count will never be decremented. It would result in a
        // memory leak and is prevented by decrementing the strong count here.
        //
        // SAFETY: The Arc is stored at least in the current task variable (set above)
        // and should also be in the task list and in the run queue. So, decrementing the
        // strong count here is safe and should not result in a use-after-free.
        Arc::decrement_strong_count(Arc::as_ptr(&next));

        x86_64::thread::jump_to(&mut next.thread().lock());
    }

    /// Schedule the current thread
    ///
    /// # Safety
    /// This function is unsafe because it relies on heavy unsafe code to switch threads, for
    /// example, it manually decrements the strong count of some Arcs, manually unlocks mutexes
    /// and calls some assembly functions to switch threads.
    /// In addition, the caller must ensure that scheduling the current thread is safe and will
    /// not cause any undefined behavior (especially deadlocks or race conditions). In most
    /// cases, this function is unsound to call in the kernel and must not be used.
    ///
    /// # Panics
    /// This function should never panic in normal conditions. However, it performs some checks
    /// to ensure that the scheduler is in a valid state, and if it is not the case, it will
    /// panic. This include checking that there is an current task, detecting if the current
    /// and the next task are an invalid strong count, etc.
    unsafe fn schedule(&self) {
        let current = self.current_task();
        if current.state() == task::State::Running {
            current.change_state(task::State::Ready);
        }

        let task = self.pick_next();
        self.set_current_task(Arc::clone(&task));

        // If the next thread is the same as the current one, we do not need to switch threads
        if current.id() != task.id() {
            log::debug!("Switching from {:?} to {:?}", current.id().0, task.id().0);

            // Here, we must force the unlocking of the current thread, acquired by the
            // scheduler when it was resumed and of the next thread, acquired by the scheduler
            // when it was suspended. We can consider that as as an advance on the use of locks,
            // since the place where the lock was acquired will never be reached again or will
            // forget about the lock (for an example, see below at the end of this function).

            // This is possible that the next thread is not locked if it was never ran before,
            // but in this case, the force_unlock function will do nothing.
            //
            // SAFETY: This is safe, but only if this function is the only place where the
            // thread are acquired and released, excepted for a few functions in the thread.rs
            // module, that should only be called by this function.
            current.thread().force_unlock();
            task.thread().force_unlock();

            // Here, we manually decrement the strong count of the next task. This is needed
            // because when we switch to the next task, this is not guaranteed that it will
            // be rescheduled (for example, if the task exits), and if it is not rescheduled,
            // the strong count will not be decremented at the end of the this function. It
            // would result in a memory leak, because the strong count would never reach 0.
            //
            // SAFETY: The Arc is stored at least in the  current task variable (set above)
            // and should also be in the task list and in the run queue. So, decrementing the
            // strong count here is safe andthe task will not be freed while we are using it.
            debug_assert!(Arc::strong_count(&task) > 1);
            Arc::decrement_strong_count(Arc::as_ptr(&task));

            let mut next = task.thread().lock();
            match current.state() {
                // If the current task is exiting, we call a special function to exit the task
                // that will do the necessary cleanup in free the memory used by the task before
                // switching to the next task.
                task::State::Exited => x86_64::thread::exit(current, &mut next),

                // If the current task is blocked or ready, we do not need to do anything
                task::State::Blocked | task::State::Ready => (),

                // Other states are not supposed to be scheduled and it is a bug if we are
                // here. We panic in this case, because this is a bug in the kernel that
                // we cannot recover from and should be fixed.
                _ => unreachable!("scheduler: invalid task state"),
            }

            // The strong count of the current task is decremented here and not above with
            // the other one because if the current task is exiting, it could be the last strong
            // reference to the task, and decrementing the strong count before calling the
            // exit function could cause an use after free. So, we decrement the strong count
            // here because it cannot be reached if the current task is exiting.
            debug_assert!(Arc::strong_count(&current) > 1);
            Arc::decrement_strong_count(Arc::as_ptr(&current));

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
            core::mem::forget(prev);

            // Here, since we already decremented the strong count of current and task, we
            // must not decrement it again to avoid undefined behavior and a potential
            // double free.
            core::mem::forget(current);
            core::mem::forget(task);
        }
    }
}

/// Setup the scheduler
#[init]
pub fn setup() {}

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
    SCHEDULER.schedule();
}

/// Engage the current CPU in the scheduler.
///
/// # Safety
/// This function is unsafe for the same
pub unsafe fn engage_cpu() -> ! {
    SCHEDULER.engage_cpu()
}

/// Return the current task running on the CPU.
///
/// # Panics
/// This function panics if there is no current task. This should never happen, excepted
/// if this called during kernel initialization.
pub fn current_task() -> Arc<Task> {
    SCHEDULER.current_task()
}

/// Return a task from its identifier if it exists, or `None` otherwise.
pub fn task(tid: task::Identifier) -> Option<Arc<Task>> {
    SCHEDULER.task(tid)
}
