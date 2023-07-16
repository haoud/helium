use super::{State, Task};
use crate::user::scheduler;
use alloc::{collections::VecDeque, sync::Arc};

pub struct WaitQueue {
    tasks: VecDeque<Arc<Task>>,
}

impl WaitQueue {
    /// Create a new empty wait queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new(),
        }
    }

    /// Add the current task to the wait queue and block it, pausing its execution
    /// and allowing other tasks to run. The task will not be resumed until another
    /// task sets its state to `State::Ready`.
    pub fn sleep(&mut self) {
        let current = scheduler::current_task();
        current.change_state(State::Blocked);

        self.tasks.push_back(Arc::clone(&current));
        unsafe {
            scheduler::schedule();
        }

        // If we get here, we have been woken up by another task. We must make sure that the
        // task is not in the wait queue anymore because it may have been woken up by another
        // method that the `wake_up_someone` method, for example, when receiving a signal.
        self.tasks.retain(|task| task.id() != current.id());
    }

    /// Wake up a blocked task in the wait queue. If there is no blocked task in the
    /// wait queue, this function does nothing.
    pub fn wake_up_someone(&mut self) -> Option<Arc<Task>> {
        // Pop tasks until we find one that is blocked or until the wait queue is empty.
        // We need to check if the task is blocked because it may have been woken up by another
        // method that the `wake_up_someone` method, for example, when receiving a signal.
        while let Some(task) = self.tasks.pop_front() {
            if task.state() == State::Blocked {
                task.change_state(State::Ready);
                return Some(task);
            }
        }
        None
    }

    /// Check if the wait queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Get the number of tasks in the wait queue.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tasks.len()
    }
}

impl Default for WaitQueue {
    fn default() -> Self {
        Self::new()
    }
}
