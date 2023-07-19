use super::{SyscallError, SyscallValue};
use crate::{
    time::{timer::Timer, uptime_fast, Nanosecond},
    user::{scheduler, task},
};

/// Exit the current task with the given exit code. The task will be terminated and
/// will not be scheduled again. If there is no other reference to the task, it will
/// be deallocated.
///
/// # Panics
/// This function panics if the current task is rescheduled after it has exited.
pub fn exit(code: usize) -> ! {
    log::debug!(
        "Task {} exited with code {}",
        scheduler::current_task().id(),
        code
    );
    scheduler::terminate(code as u64);
    scheduler::reschedule();
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
        task::sleep();
    }
    Ok(0)
}

/// Yield the CPU to another task. If there is no other task ready to run or if there
/// is only lower priority tasks, the current task will continue to run.
///
/// # Errors
/// This function will never return an error, but it is declared as returning a `Result`
/// to be consistent with the other syscalls. It always returns `0`.
pub fn yields() -> Result<SyscallValue, SyscallError> {
    scheduler::yield_cpu();
    Ok(0)
}
