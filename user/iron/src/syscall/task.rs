use super::{syscall_return, SyscallString, Errno};
use crate::syscall::Syscall;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum SpawnError {
    NoSuchSyscall = 1,
    BadAddress,
    IoError,
    InvalidElf,
    OutOfMemory,
    UnknownError,
}

impl From<Errno> for SpawnError {
    fn from(error: Errno) -> Self {
        if error.code() > Self::UnknownError as isize {
            unsafe { core::mem::transmute(error) }
        } else {
            Self::UnknownError
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier(pub u64);

/// Obtain the current task ID.
///
/// A task ID is a unique identifier for a task. Each task has a unique task ID, and unlike
/// UNIX process IDs, task IDs are never reused. The first task created has a task ID of 0,
/// and each subsequent task has a task ID that is incremented by 1.
pub fn id() -> u64 {
    let id;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::TaskId as u64,
            lateout("rax") id,
        );
    }
    id
}

/// Sleep for the specified number of nanoseconds. The current task will be suspended for
/// at least the specified number of nanoseconds, but may be suspended for longer due to
/// the way the scheduler works and because the timer granularity is often not very fine.
pub fn nanosleep(nano: u64) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::TaskSleep as u64,
            in("rsi") nano,
        );
    }
}

/// Sleep for the specified number of seconds. The current task will be suspended for
/// at least the specified number of seconds, but may be suspended for longer due to
/// the way the scheduler works and because the timer granularity is often not very fine.
pub fn sleep(sec: u64) {
    nanosleep(sec * 1000000000)
}

/// Yield the current task. The current task will be suspended and another task will be
/// scheduled to run. If there are no other tasks to run, the current task will continue
/// running.
///
/// This should be called when no more work can be done by the current task to avoid
/// wasting CPU cycles.
pub fn yields() {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::TaskYield as u64,
        );
    }
}

/// Terminates the current process with the specified exit code. This function will never
/// return and will immediately terminate the current process. Because this function never
/// returns, and that it terminates the process, no destructors on the current stack will
/// be run.
pub fn exit(code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::TaskExit as u64,
            in("rsi") code,
            options(noreturn)
        );
    }
}

/// Spawn a new task from the specified ELF file.
pub fn spawn(path: &str) -> Result<Identifier, SpawnError> {
    let str = SyscallString::from(path);
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::TaskSpawn as u64,
            in("rsi") &str as *const _ as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(SpawnError::from(errno)),
        Ok(ret) => Ok(Identifier(ret as u64)),
    }
}
