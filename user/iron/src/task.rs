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

pub fn nanosleep(nano: u64) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 4,
            in("rsi") nano,
        );
    }
}

pub fn sleep(sec: u64) {
    nanosleep(sec * 1000000000)
}
