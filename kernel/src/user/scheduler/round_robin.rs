use super::task::{self, State, Task};
use super::{yield_cpu, SCHEDULER};
use alloc::{sync::Arc, vec::Vec};
use core::cell::RefCell;
use macros::per_cpu;
use sync::{Lazy, Spinlock};

/// The current task running on the CPU. This is a per-CPU variable, so each CPU has its own
/// current task. If the CPU is idle, this variable is set to `None`.
#[per_cpu]
pub static CURRENT_TASK: RefCell<Option<Arc<Task>>> = RefCell::new(None);

/// The idle task associated with the CPU. This is a per-CPU variable, so each CPU has its own
/// idle task. The idle task is a task that is always ready to run and is used when no other
/// task are ready to run.
#[per_cpu]
pub static IDLE_TASK: Lazy<Arc<Task>> = Lazy::new(Task::idle);

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

    /// Create a new round-robin scheduler. This function returns a new round-robin scheduler
    /// with an empty run list.
    #[must_use]
    pub fn new() -> Self {
        Self {
            run_queue: Spinlock::new(Vec::new()),
        }
    }

    /// Find a task to run. This function will return the first task in the run list that is
    /// ready to run. If no thread is found, this function returns `None`, otherwise it sets the
    /// thread state to `Running` and returns the thread.
    /// We set the state of the thread to `Running` here to avoid a race condition where the thread
    /// could be picked by another CPU before we set its state to `Running`.
    ///
    /// # Note
    /// The returned task is guaranteed to not be an idle task. This is because idle tasks are
    /// special tasks that are always ready to run, and are only picked when no other task is
    /// ready to run. Returning them here would not make sense.
    fn pick_task(&self) -> Option<Arc<Task>> {
        let run_queue = self.run_queue.lock();
        run_queue
            .iter()
            .filter(|t| t.quantum > 0)
            .filter(|t| !t.task.priority().is_idle())
            .find(|t| t.task.state().executable())
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
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Scheduler for RoundRobin {
    /// Returns the current task running on the CPU
    ///
    /// # Panics
    /// This function panics if no task is running on the CPU. This should never happen and
    /// indicates a bug in the kernel.
    fn current_task(&self) -> Arc<Task> {
        CURRENT_TASK
            .local()
            .borrow()
            .as_ref()
            .map(Arc::clone)
            .expect("No task running on the CPU")
    }

    /// Sets the task passed as argument as the current task running on the CPU and changes its
    /// state to `Running`. The Arc counter of the previous task is decremented in this function
    /// and can be dropped if the counter reaches 0 (take care of this !)
    fn set_current_task(&self, task: Arc<Task>) {
        CURRENT_TASK
            .local()
            .borrow_mut()
            .insert(task)
            .change_state(State::Running);
    }

    /// Picks the next task to run. This function will first try to find a task to run. If no
    /// task is found, it will redistribute quantum to all tasks and try again. If no task is
    /// found again, it will return the idle task.
    ///
    /// FIXME: Due to some limitations in the current implementation, this function will never
    /// return the current task, even if it is the only task ready to run. This is because the
    /// scheduler cannot handle this case and panic for obscure reasons. This should be fixed
    /// in the future.
    /// TODO: Is the above still true ? Not sure about that...
    fn pick_next(&self) -> Arc<Task> {
        self.pick_task()
            .or_else(|| {
                self.redistribute();
                self.pick_task()
            })
            .unwrap_or_else(|| Arc::clone(&IDLE_TASK.local()))
    }

    /// Adds a task to the run list
    ///
    /// # Panics
    /// This function panics if a task with the same identifier is already in the run list. This
    /// should never happen and indicates a bug in the kernel.
    fn add_task(&self, task: Arc<Task>) {
        assert!(self.find_task(task.id()).is_none());
        self.run_queue.lock().push(RunnableTask {
            quantum: Self::DEFAULT_QUANTUM,
            task,
        });
    }

    /// Removes a task from the run list. If the task is not found in the run list, this function
    /// does nothing.
    fn remove_task(&self, tid: task::Identifier) {
        self.run_queue.lock().retain(|t| t.task.id() != tid);
    }

    /// Returns a task from the run list by its identifier. This function returns `None` if no
    /// task with the given identifier is found.
    fn find_task(&self, tid: task::Identifier) -> Option<Arc<Task>> {
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
        let reschedule = {
            let mut run_queue = self.run_queue.lock();
            let current = SCHEDULER.current_task();
            let running = run_queue
                .iter_mut()
                .find(|t| t.task.id() == current.id())
                .expect("Current task not found in run queue");

            running.quantum = running.quantum.saturating_sub(1);
            running.quantum == 0 || current.priority().is_idle()
        };

        if reschedule {
            // SAFETY: TODO
            unsafe { yield_cpu() };
        }
    }
}
