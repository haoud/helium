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
            in("rax") 1,
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
            in("rax") 4,
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
            in("rax") 5,
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
            in("rax") 0,
            in("rsi") code,
            options(noreturn)
        );
    }
}
