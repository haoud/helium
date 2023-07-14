use super::{SyscallError, SyscallValue};
use crate::user::{scheduler, task};
use tap::Tap;

/// Exit the current task with the given exit code. Actually, this function just exit the task,
/// and the task will not be destroyed until the `TASK_DESTROY` syscall is called.
///
/// # Panics
/// This function panics if the current task is rescheduled after it has exited.
pub fn exit(code: u64) -> ! {
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
pub fn id() -> Result<SyscallValue, SyscallError> {
    Ok(scheduler::current_task().id().0)
}
