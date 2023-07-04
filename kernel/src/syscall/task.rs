use tap::Tap;
use super::SyscallReturn;
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

/// Return the identifier of the current task.
///
/// # Panics
/// This function panics if there is no current task running on the CPU (which should
/// never happen and is a bug).
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub fn id() -> SyscallReturn {
    SyscallReturn::from(scheduler::current_task().id().0)
}
