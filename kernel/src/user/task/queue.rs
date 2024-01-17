use super::{sleep, State, Task};
use crate::user::scheduler::{Scheduler, SCHEDULER};
use alloc::collections::VecDeque;

pub struct WaitQueue {
    tasks: Spinlock<VecDeque<Arc<Task>>>,
}

impl WaitQueue {
    /// Create a new empty wait queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tasks: Spinlock::new(VecDeque::new()),
        }
    }

    /// Add the current task to the wait queue and block it, pausing its execution
    /// and allowing other tasks to run. The task will not be resumed until another
    /// task sets its state to `State::Ready`.
    pub fn sleep(&self) {
        let current = SCHEDULER.current_task();
        let id = current.id();

        self.tasks.lock().push_back(current);
        sleep();

        // If we get here, we have been woken up by another task. We must make sure that the
        // task is not in the wait queue anymore because it may have been woken up by another
        // method that the `wake_up_someone` method, for example, when receiving a signal.
        self.tasks.lock().retain(|task| task.id() != id);
    }

    /// Wake up a blocked task in the wait queue. If there is no blocked task in the
    /// wait queue, this function does nothing.
    pub fn wake_up_someone(&self) -> Option<Arc<Task>> {
        // Pop tasks until we find one that is blocked or until the wait queue is empty.
        // We need to check if the task is blocked because it may have been woken up by another
        // method that the `wake_up_someone` method, for example, when receiving a signal.
        while let Some(task) = self.tasks.lock().pop_front() {
            if task.state() == State::Blocked {
                task.change_state(State::Ready);
                return Some(task);
            }
        }
        None
    }
}

impl Default for WaitQueue {
    fn default() -> Self {
        Self::new()
    }
}
