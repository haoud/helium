use crate::task::{self, Task};
use alloc::sync::Arc;
use sync::Lazy;

use self::round_robin::RoundRobin;

pub mod round_robin;

static SCHEDULER: Lazy<RoundRobin> = Lazy::new(RoundRobin::default);

/// A trait that represents a scheduler. This trait is used to abstract the scheduler
/// implementation, allowing us to easily switch between different schedulers.
pub trait Scheduler {
    /// Return the current task running on the CPU. This function should return `None` if no task
    /// is currently running on the CPU (meaning that the CPU is idle)
    fn current_task(&self) -> Option<Arc<Task>>;

    /// Set the current task running on the CPU, and set the task state to `Running`.
    fn set_current_task(&self, task: Arc<Task>);

    /// Pick the next thread to run If no thread is available, this function should wait until a
    /// thread is available.
    /// Note that this function can return the current thread, and this case should be correctly
    /// handled by the caller.
    fn pick_next(&self) -> Arc<Task>;

    /// Add a task to the scheduler. The task will be added to the run queue, and will be
    /// available to be run by the scheduler.
    fn add_task(&self, thread: Task);

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

        // If the next thread is the same as the current one,
        // we do not need to switch threads (obviously)
        if current.id() != next.id() {
            match current.state() {
                task::State::Running => current.change_state(task::State::Ready),
                task::State::Blocked | task::State::Terminated => (),
                _ => unreachable!(),
            }

            // TODO: Explain why we need to force unlock the thread lock
            next.thread().force_unlock();

            let mut prev = current.thread().lock();
            let mut next = next.thread().lock();
            x86_64::thread::switch(&mut prev, &mut next);
            core::mem::forget(next);
        }
    }
}

pub fn setup() {
    Lazy::force(&SCHEDULER);
}

pub fn add_task(task: Task) {
    SCHEDULER.add_task(task);
}

pub fn remove_task(tid: task::Identifier) {
    SCHEDULER.remove_task(tid);
}

pub fn timer_tick() {
    SCHEDULER.timer_tick();
}

pub unsafe fn schedule() {
    SCHEDULER.schedule();
}

pub fn current_task() -> Option<Arc<Task>> {
    SCHEDULER.current_task()
}

pub fn task(tid: task::Identifier) -> Option<Arc<Task>> {
    SCHEDULER.task(tid)
}
