use super::{SyscallError, SyscallReturn};
use crate::user::{scheduler, task};

/// Exit the current task with the given exit code. Actually, this function just exit the task,
/// and the task will not be destroyed until the `TASK_DESTROY` syscall is called.
///
/// # Panics
/// This function panics if there is no current task running on the CPU, or if the current task
/// is rescheduled after it has exited.
pub fn exit(code: u64) -> ! {
    let current = scheduler::current_task().unwrap();
    current.change_state(task::State::Terminated);

    log::debug!("Task {} exited with code {}", current.id(), code);
    unsafe {
        scheduler::schedule();
    }

    unreachable!("Task should never be scheduled again after exiting");
}

/// Destroy an task that has exited by its identifier. This function will actually destroy the
/// task and free all its resources.
/// We cannot destroy the task immediately after it has exited because the task may have some
/// ressources that are still in use. The most common example is the task kernel stack: it is
/// still in use until the task is scheduled again and the kernel stack is switched. It would
/// greately complicate the kernel code to handle this case, so we rely on other tasks to call*
/// this function when they are done with the task.
///
/// # Errors
/// This function returns `SyscallError::TaskNotFound` if the task does not exist, or
/// `SyscallError::TaskInUse` if the task has not exited yet.
pub fn destroy(tid: u64) -> Result<SyscallReturn, SyscallError> {
    let tid = task::Identifier::new(tid);

    if let Some(task) = task::get(tid) {
        if task.state() != task::State::Terminated {
            return Err(SyscallError::TaskInUse);
        }
        scheduler::remove_task(tid);
        task::destroy(tid);
        return Ok(0);
    }

    Err(SyscallError::TaskNotFound)
}

/// Return the identifier of the current task.
///
/// # Panics
/// This function panics if there is no current task running on the CPU.
pub fn handle() -> Result<i64, SyscallError> {
    Ok(scheduler::current_task().unwrap().id().0 as i64)
}
