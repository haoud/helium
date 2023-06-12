use macros::syscall_handler;

pub mod task;

/// A syscall return value must be compatible with i64, as it will be returned in the rax
/// register for the userland code.
pub type SyscallReturn = i64;

// A struct that contains all the syscall numbers. This struct is used to avoid
// using magic numbers in the syscall handler.
pub struct Syscall;
impl Syscall {
    pub const TASK_EXIT: u64 = 0;
    pub const TASK_DESTROY: u64 = 1;
}

/// A struct that contains all the possible syscall errors. When a syscall returns, it can
/// return any value that fit in an i64, but values between -1 and -4095 are reserved for
/// indicating an error. This works similarly to errno in Linux.
#[repr(i64)]
pub enum SyscallError {
    NoSuchSyscall = 1,
    InvalidArgument = 2,
    TaskNotFound = 3,
    TaskInUse = 4,
}

/// Handle a syscall. This function is called from the syscall interrupt handler (written in
/// assembly) and is responsible for dispatching the syscall to the appropriate handler within
/// the kernel.
#[syscall_handler]
#[allow(unused_variables)]
fn syscall(syscall: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i64 {
    let result = match syscall {
        Syscall::TASK_EXIT => task::exit(arg1),
        Syscall::TASK_DESTROY => task::destroy(arg1),
        _ => panic!("Unknown syscall {}", syscall),
    };
    result.unwrap_or_else(|e| -(e as i64))
}
