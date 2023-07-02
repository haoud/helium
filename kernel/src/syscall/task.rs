use tap::Tap;

use super::{SyscallError, SyscallReturn};
use crate::user::{scheduler, task};

/// Exit the current task with the given exit code. Actually, this function just exit the task,
/// and the task will not be destroyed until the `TASK_DESTROY` syscall is called.
///
/// # Panics
/// This function panics if the current task is rescheduled after it has exited.
pub fn exit(code: u64) -> ! {
    let id = scheduler::current_task()
        .tap(|task| task.change_state(task::State::Exited))
        .id();

    log::debug!("Task {} exited with code {}", id, code);
    scheduler::remove_task(id);
    task::remove(id);

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
#[must_use]
pub fn destroy(tid: u64) -> SyscallReturn {
    let tid = task::Identifier::new(tid);

    if let Some(task) = task::get(tid) {
        if task.state() != task::State::Exited {
            return SyscallReturn::failure(SyscallError::TaskInUse);
        }
        task.change_state(task::State::Terminated);
        scheduler::remove_task(tid);
        task::remove(tid);
        return SyscallReturn::success();
    }

    SyscallReturn::failure(SyscallError::TaskNotFound)
}

/// Return the identifier of the current task.
///
/// # Errors
/// This function returns the identifier of the current task and should never fail.
///
/// # Panics
/// This function panics if there is no current task running on the CPU (which should
/// never happen and is a bug).
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub fn handle() -> SyscallReturn {
    SyscallReturn::from(scheduler::current_task().id().0)
}
