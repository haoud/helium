use self::round_robin::RoundRobin;
use super::task::{self, State, Task};
use crate::x86_64;

pub mod round_robin;

/// The scheduler used by the kernel. This is a global variable to allow changing the scheduler
/// implementation at compile time more easily.
pub static SCHEDULER: Lazy<RoundRobin> = Lazy::new(RoundRobin::new);

/// The last task that was saved on the current CPU. This is used to unlock the spinlock of the
/// thread after the context switch, to avoid deadlocks. See the `unlock_saved_thread` function
/// for more information.
#[per_cpu]
static mut SAVED_TASK: Option<Arc<Task>> = None;

/// A trait that represents a scheduler. This trait is used to abstract the scheduler
/// implementation, allowing us to easily switch between different schedulers.
pub trait Scheduler {
    /// Return the current task running on the CPU.
    fn current_task(&self) -> Arc<Task>;

    /// Set the current task running on the CPU, and set the task state to `Running`.
    fn set_current_task(&self, task: Arc<Task>);

    /// Pick the next thread to run if no thread is available, this function should wait until a
    /// thread is available. Note that this function can return the current thread, and this case
    /// should be correctly handled by the caller.
    fn pick_next(&self) -> Arc<Task>;

    /// Add a task to the scheduler. The task will be added to the run queue, and will be
    /// available to be run by the scheduler.
    fn add_task(&self, task: Arc<Task>);

    /// Remove a thread from the scheduler. The thread will be removed from the run queue, and
    /// cannot be run until it is added again. If the task is currently running, this function
    /// removes it from the run queue, but does not stop it, only preventing it from being
    /// re-executed when it yields.
    fn remove_task(&self, tid: task::Identifier);

    /// Return a task from its identifier if it exists, or `None` otherwise.
    fn find_task(&self, tid: task::Identifier) -> Option<Arc<Task>>;

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
    /// This function is unsafe because it make assumption about the state of the CPU and the
    /// kernel. This function should only be called by the AP boot code, and only when the CPU
    /// is in a valid state to run a thread. Any other call to this function will result in
    /// undefined behavior.
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

    /// Schedule the current thread.
    ///
    /// A task can enter in this function with any state excepted for the `Running`
    /// state. Instead, the task state should be set the `Rescheduled` state before
    /// calling this function.
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
    /// and the next task are an invalid strong count, verify that preemption is enabled and
    /// that interrupts are disabled...
    unsafe fn schedule(&self) {
        assert!(!x86_64::irq::enabled());
        assert!(task::preempt::enabled());

        let current_task = self.current_task();
        assert!(current_task.state() != task::State::Running);
        let next_task = self.pick_next();

        // If the next thread is the same as the current one, we do not need to switch threads
        if current_task.id() == next_task.id() {
            current_task.change_state(task::State::Running);
            return;
        }

        self.set_current_task(Arc::clone(&next_task));

        // Here, we manually decrement the strong count of the next task. This is needed
        // because when we switch to the next task, this is not guaranteed that it will
        // be rescheduled (for example, if the task exits), and if it is not rescheduled,
        // the strong count will not be decremented at the end of the this function. It
        // would result in a memory leak, because the strong count would never reach 0.
        //
        // SAFETY: The Arc is stored at least in the current task variable (set above), in
        // the task list and in the run queue. So, decrementing the strong count here is safe
        // and the task will not be freed while we are using it.
        debug_assert!(Arc::strong_count(&next_task) > 1);
        Arc::decrement_strong_count(Arc::as_ptr(&next_task));

        let mut next_thread = next_task.thread().lock();
        match current_task.state() {
            // If the current task is exiting, we call a special function to exit the task
            // that will do the necessary cleanup in free the memory used by the task before
            // switching to the next task.
            task::State::Terminated => {
                x86_64::thread::exit(current_task, &mut next_thread)
            }

            // If the current task is rescheduled, we change its state to ready
            task::State::Rescheduled => {
                current_task.change_state(task::State::Ready)
            }

            // If the current task is blocked, we do not need to do anything
            task::State::Blocked => (),

            // Other states are not supposed to be scheduled and it is a bug if we are
            // here. We panic in this case, because this is a bug in the kernel that
            // we cannot recover from and should be fixed.
            _ => unreachable!(
                "scheduler: invalid task state {:#?}",
                current_task.state()
            ),
        }

        // The strong count of the current task is decremented here and not above with
        // the other one because if the current task is exiting, it could be the last strong
        // reference to the task, and decrementing the strong count before calling the
        // exit function could cause an use after free. So, we decrement the strong count
        // here because it cannot be reached if the current task is exiting.
        //
        // SAFETY: The Arc is stored at least in the task list and in the run queue. So,
        // decrementing the strong count here is safe and the task will not be freed while
        // we are using it.
        debug_assert!(Arc::strong_count(&current_task) > 1);
        Arc::decrement_strong_count(Arc::as_ptr(&current_task));

        // Store the task that will be saved in a global variable to allow unlock it
        // after the contexte switch to avoid deadlocks, since some of the code called
        // is written in assembly and cannot drop a lock guard.
        *SAVED_TASK.local_mut() = Some(Arc::clone(&current_task));

        let mut prev_thread = current_task.thread().lock();
        x86_64::thread::switch(&mut prev_thread, &mut next_thread);

        // Unlock the previously saved thread.
        unlock_threads();

        // We must forget the lock guard, because there was already manually unlocked
        // and we must not unlock it again
        core::mem::forget(next_thread);
        core::mem::forget(prev_thread);

        // Here, since we already decremented the strong count of `current` and `task`, we
        // must not decrement it again to avoid undefined behavior and a potential
        // double free.
        core::mem::forget(current_task);
        core::mem::forget(next_task);
    }
}

/// Setup the scheduler
#[init]
pub fn setup() {}

/// Yield the current thread. If preemption is disabled, this function prints a warning
/// message and does nothing.
///
/// # Safety
/// This function is unsafe because the caller must ensure that the kernel is in a valid
/// state to yield the current thread. In most cases, this function is unsound to call in
/// the kernel and must not be used.
pub unsafe fn yield_cpu() {
    if task::preempt::enabled() {
        SCHEDULER.current_task().change_state(State::Rescheduled);
        SCHEDULER.schedule();
    } else {
        log::warn!("scheduler: yield_cpu called with preemption disabled");
    }
}

/// Terminate the current task. It change the state of the current task to `Terminated`
/// and remove it from the scheduler and from the task list
pub fn terminate(_code: u64) {
    let current_task = SCHEDULER.current_task();
    current_task.change_state(State::Terminated);

    let tid = current_task.id();
    SCHEDULER.remove_task(tid);
    task::remove(tid);
}

/// Unlock the threads that was involved in the last context switch (the current thread
/// and the previous thread)
/// This is needed because how the scheduler is implemented: the code that switch the
/// threads is written in assembly and cannot drop a lock guard. So, we must manually
/// unlock the threads after the context switch. Otherwise, those threads will remain
/// locked, and it will cause a deadlock sooner or later.
///
/// # Safety
/// This function must be called only after a context switch and only once, otherwise
/// it will cause undefined behavior.
#[no_mangle]
unsafe extern "C" fn unlock_threads() {
    // SAFETY: This is safe because this function must be called just after the
    // context switch, and therefore the current thread and the previous thread
    // are still locked, but not used anymore. So, we can safely unlock them.
    SCHEDULER.current_task().thread().force_unlock();
    if let Some(saved) = SAVED_TASK.local_mut().take() {
        saved.thread().force_unlock();
    }
}
