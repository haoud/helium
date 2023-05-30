use macros::syscall_handler;
use task_exit::task_exit;

pub mod task_exit;

pub struct Syscall;

impl Syscall {
    pub const TASK_EXIT: u64 = 0;
}

#[repr(i64)]
pub enum SyscallResult {
    Ok = 0,
    NoSuchSyscall = -1,
}

#[syscall_handler]
#[allow(unused_variables)]
pub fn syscall(syscall: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i64 {
    match syscall {
        Syscall::TASK_EXIT => task_exit(arg1) as i64,
        _ => panic!("Unknown syscall {}", syscall),
    }
}
