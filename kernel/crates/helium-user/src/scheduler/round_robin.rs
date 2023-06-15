use super::{current_task, schedule};
use crate::task::{self, State, Task};
use alloc::{sync::Arc, vec::Vec};
use core::cell::{RefCell};
use macros::per_cpu;
use sync::Spinlock;

/// The current task running on the CPU. This is a per-CPU variable, so each CPU has its own
/// current task. If the CPU is idle, this variable is set to `None`.
#[per_cpu]
pub static CURRENT_TASK: RefCell<Option<Arc<Task>>> = RefCell::new(None);

/// A task that can be run by the scheduler. This structure contains a task and its quantum. The
/// quantum is the number of ticks that the task can run before being forced to be preempted.
pub struct RunnableTask {
    task: Arc<Task>,
    quantum: usize,
}

/// A round-robin scheduler. This scheduler is a simple scheduler that runs each thread for a
/// certain amount of time before switching to the next thread. This scheduler is not preemptive,
/// and relies on the timer interrupt to switch between threads.
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

    /// Called when no other task is ready to run. This will simply wait for the next interrupt
    /// and return. This is the caller responsibility to check if there is a task ready to run
    /// after this function returns.
    fn idle(&self) {
        // We are idle and no task is ready to run, so we set the current task to
        //`None` and wait for the next interrupt.
        // We still need to save the current task because it will be restored when
        // an interrupt will occur because wwe still are in the task context.
        let current = CURRENT_TASK.local().borrow_mut().take().unwrap();
        unsafe {
            x86_64::cpu::wait_for_interrupt();
        }
        CURRENT_TASK.local().borrow_mut().replace(current);
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
    /// Returns the current task running on the CPU. This function returns `None` if the CPU is
    /// idle.
    fn current_task(&self) -> Option<Arc<Task>> {
        CURRENT_TASK.local().borrow().as_ref().map(Arc::clone)
    }

    /// Sets the current task running on the CPU and set its state to `Running`.
    fn set_current_task(&self, task: Arc<Task>) {
        task.change_state(State::Running);
        CURRENT_TASK.local().borrow_mut().replace(task);
    }

    /// Picks the next task to run. This function will first try to find a task to run. If no
    /// task is found, it will redistribute quantum to all tasks and wait for the next interrupt.
    /// If no task is ready to run yet, it will wait for the next interrupt and then retry until
    /// a task is found.
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

    /// Adds a task to the run list
    fn add_task(&self, task: Arc<Task>) {
        self.run_queue.lock().push(RunnableTask {
            quantum: Self::DEFAULT_QUANTUM,
            task,
        });
    }

    /// Removes a task from the run list
    fn remove_task(&self, tid: task::Identifier) {
        self.run_queue.lock().retain(|t| t.task.id() != tid);
    }

    /// Returns a task from the run list by its identifier. This function returns `None` if no
    /// task with the given identifier is found.
    fn task(&self, tid: task::Identifier) -> Option<Arc<Task>> {
        self.run_queue
            .lock()
            .iter()
            .find(|t| t.task.id() == tid)
            .map(|t| Arc::clone(&t.task))
    }

    /// Called when the timer interrupt occurs. This function will decrement the quantum of the
    /// current task. If the quantum reaches 0, it will call `schedule` to switch to the next
    /// task.
    /// If there is no current task, this either means that the CPU is idle or that the CPU has
    /// not yet been engaged in the scheduler. In this case, we call `run_ap` to run the a task
    /// on the CPU.
    fn timer_tick(&self) {
        unsafe {
            if let Some(current) = current_task() {
                let mut run_queue = self.run_queue.lock();
                let running = run_queue
                    .iter_mut()
                    .find(|t| Arc::ptr_eq(&t.task, &current))
                    .unwrap();
    
                running.quantum -= 1;
                if running.quantum == 0 {
                    schedule();
                }
            } else if self.engaged() {
                self.engage_cpu();
            }
        }
    }
}
