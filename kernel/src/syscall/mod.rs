use macros::syscall_handler;

pub mod task;

/// A syscall return value must be compatible with i64, as it will be returned in the rax
/// register for the userland code.
pub type SyscallReturn = i64;

// A struct that contains all the syscall numbers used by the kernel.
#[non_exhaustive]
#[repr(u64)]
pub enum Syscall {
    TaskExit = 0,
    TaskDestroy = 1,
    TaskHandle = 2,
    Last,
}

impl Syscall {
    /// Create a new Syscall from a u64. If the u64 is not a valid syscall number, it
    /// returns None.
    #[must_use]
    pub fn from(id: u64) -> Option<Syscall> {
        if id < Self::Last as u64 {
            Some(unsafe { core::mem::transmute(id) })
        } else {
            None
        }
    }
}

/// A struct that contains all the possible syscall errors. When a syscall returns, it can
/// return any value that fit in an i64, but values between -1 and -4095 are reserved for
/// indicating an error. This works similarly to errno in Linux.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum SyscallError {
    NoSuchSyscall = 1,
    InvalidArgument = 2,
    TaskNotFound = 3,
    TaskInUse = 4,
}

impl SyscallError {
    /// Return the errno value of the current error. A syscall can return any value that
    /// fits in an i64, but values between -1 and -4095 are reserved for indicating an
    /// error, so we just convert the error to its negative value. This works similarly
    /// to errno in Linux.
    #[must_use]
    pub fn errno(&self) -> i64 {
        -(*self as i64)
    }
}

/// Handle a syscall. This function is called from the syscall interrupt handler, written in
/// assembly and is responsible for dispatching the syscall to the appropriate handler within
/// the kernel.
#[syscall_handler]
#[allow(unused_variables)]
fn syscall(id: u64, a: u64, b: u64, c: u64, d: u64, e: u64) -> i64 {
    match Syscall::from(id) {
        Some(Syscall::TaskExit) => task::exit(a),
        Some(Syscall::TaskDestroy) => task::destroy(a),
        Some(Syscall::TaskHandle) => task::handle(),

        Some(Syscall::Last) | None => Err(SyscallError::NoSuchSyscall),
    }
    .unwrap_or_else(|e| e.errno())
}
