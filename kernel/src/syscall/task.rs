use super::{SyscallError, SyscallValue};
use crate::{
    time::{timer::Timer, uptime_fast, Nanosecond},
    user::{
        scheduler,
        task::{self, queue::WaitQueue},
    },
};
use tap::Tap;

/// Exit the current task with the given exit code. Actually, this function just exit the task,
/// and the task will not be destroyed until the `TASK_DESTROY` syscall is called.
///
/// # Panics
/// This function panics if the current task is rescheduled after it has exited.
pub fn exit(code: usize) -> ! {
    let id = scheduler::current_task()
        .tap(|task| task.change_state(task::State::Terminated))
        .id();

    log::debug!("Task {} exited with code {}", id, code);
    scheduler::remove_task(id);
    task::remove(id);

    unsafe {
        scheduler::schedule();
    }
    unreachable!("Task should never be scheduled again after exiting");
}

/// Return the identifier of the current task.
///
/// # Errors
/// This function will never return an error, but it is declared as returning a `Result` to
/// be consistent with the other syscalls.
///
/// # Panics
/// This function panics if there is no current task running on the CPU (which should
/// never happen and is a bug).
#[allow(clippy::cast_possible_truncation)]
pub fn id() -> Result<SyscallValue, SyscallError> {
    Ok(scheduler::current_task().id().0 as usize)
}

/// Put the current task to sleep for at least the given number of nanoseconds. The task 
/// will be woken up when the timer expires. Due to the way the timer system works, the 
/// task may not be woken up immediately after the timer expires and may be delayed by 
/// a few milliseconds.
/// 
/// # Errors
/// This function will never return an error, but it is declared as returning a `Result`
/// to be consistent with the other syscalls. It always returns `0`.
pub fn sleep(nano: usize) -> Result<SyscallValue, SyscallError> {
    let expiration = uptime_fast() + Nanosecond::new(nano as u64);
    
    // Create a timer that will wake up the task when it expires.
    let current = scheduler::current_task();
    let timer = Timer::new(expiration, move |_| {
        if current.state() == task::State::Blocked {
            current.change_state(task::State::Ready);
        }
    });

    // Put the task to sleep if the timer is active (i.e nanoseconds > 0)
    if timer.active() {
        WaitQueue::new().sleep();
    }
    Ok(0)
}
