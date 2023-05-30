use core::cell::RefCell;

use crate::task::{self, State, Task};
use alloc::{sync::Arc, vec::Vec};
use macros::per_cpu;
use sync::Spinlock;

use super::current_task;

#[per_cpu]
pub static CURRENT_TASK: RefCell<Option<Arc<Task>>> = RefCell::new(None);

pub struct RunnableTask {
    quantum: usize,
    task: Arc<Task>,
}

pub struct RoundRobin {
    run_queue: Spinlock<Vec<RunnableTask>>,
}

impl RoundRobin {
    pub const DEFAULT_QUANTUM: usize = 20;

    /// Find a task to run. This function will return the first task in the run list that is
    /// ready to run. If no thread is found, this function returns `None`, otherwise it sets the
    /// thread state to `Running` and returns the thread.
    /// We set the state of the thread to `Running` here to avoid a race condition where the thread
    /// could be picked by another CPU before we set its state to `Running`.
    fn pick_task(&self) -> Option<Arc<Task>> {
        self.run_queue
            .lock()
            .iter()
            .filter(|t| t.quantum > 0)
            .find(|t| t.task.state() == State::Created || t.task.state() == State::Ready)
            .map(|t| {
                t.task.change_state(State::Running);
                Arc::clone(&t.task)
            })
    }

    /// Redistribute quantum of all threads in the run list. This function is called when no
    /// thread is ready to run, and is used to redistribute quantum to all threads in the run list,
    fn redistribute(&self) {
        self.run_queue
            .lock()
            .iter_mut()
            .for_each(|t| t.quantum = Self::DEFAULT_QUANTUM);
    }

    fn idle(&self) {
        // Indicate that we are idle and no task is running
        CURRENT_TASK.local().borrow_mut().take();
        unsafe {
            x86_64::instruction::sti();
            x86_64::instruction::hlt();
            x86_64::instruction::cli();
        }
    }
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self {
            run_queue: Spinlock::new(Vec::new()),
        }
    }
}

impl super::Scheduler for RoundRobin {
    fn current_task(&self) -> Option<Arc<Task>> {
        CURRENT_TASK.local().borrow().as_ref().map(Arc::clone)
    }

    fn set_current_task(&self, task: Arc<Task>) {
        task.change_state(State::Running);
        CURRENT_TASK.local().borrow_mut().replace(task);
    }

    fn pick_next(&self) -> Arc<Task> {
        if let Some(task) = self.pick_task() {
            task
        } else {
            self.redistribute();
            loop {
                if let Some(task) = self.pick_task() {
                    break task;
                }
                self.redistribute();
                self.idle();
            }
        }
    }

    fn add_task(&self, thread: Task) {
        self.run_queue.lock().push(RunnableTask {
            quantum: Self::DEFAULT_QUANTUM,
            task: Arc::new(thread),
        });
    }

    fn remove_task(&self, tid: task::Identifier) {
        self.run_queue.lock().retain(|t| t.task.id() != tid);
    }

    fn task(&self, tid: task::Identifier) -> Option<Arc<Task>> {
        self.run_queue
            .lock()
            .iter()
            .find(|t| t.task.id() == tid)
            .map(|t| Arc::clone(&t.task))
    }

    fn timer_tick(&self) {
        if let Some(current) = current_task() {
            let mut run_queue = self.run_queue.lock();
            let running = run_queue
                .iter_mut()
                .find(|t| Arc::ptr_eq(&t.task, &current))
                .unwrap();

            running.quantum -= 1;
            if running.quantum == 0 {
                unsafe {
                    self.schedule();
                }
            }
        }
    }
}
