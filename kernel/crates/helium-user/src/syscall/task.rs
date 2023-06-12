use super::{SyscallError, SyscallReturn};
use crate::{scheduler, task};

/// Exit the current task with the given exit code. Actually, this function just exit the task,
/// and the task will not be destroyed until the `TASK_DESTROY` syscall is called.
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
pub fn destroy(tid: u64) -> Result<SyscallReturn, SyscallError> {
    let tid = task::Identifier::new(tid);

    if let Some(task) = task::get(tid) {
        if task.state() != task::State::Terminated {
            return Err(SyscallError::TaskInUse);
        }
        scheduler::remove_task(tid);
        task::destroy_task(tid);
        return Ok(0);
    }

    Err(SyscallError::TaskNotFound)
}
