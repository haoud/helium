use crate::user::buffer::BufferError;
use addr::user::InvalidUserVirtual;
use macros::syscall_handler;

pub mod serial;
pub mod task;

/// The type of the return value of a syscall. All syscalls must return a value that fits
/// in an usize. However, some values are reserved for indicating an error: values between
/// -1 and -4095 are reserved for indicating an error (see `SyscallError` for more details).
pub type SyscallValue = usize;

// A struct that contains all the syscall numbers used by the kernel.
#[non_exhaustive]
#[repr(usize)]
pub enum Syscall {
    TaskExit = 0,
    TaskId = 1,
    SerialRead = 2,
    SerialWrite = 3,
    Last,
}

impl Syscall {
    /// Create a new Syscall from a u64. If the u64 is not a valid syscall number, it
    /// returns None.
    #[must_use]
    pub fn from(id: usize) -> Option<Syscall> {
        if id < Self::Last as usize {
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
    BadAddress = 5,
    NotImplemented = 6,
}

impl SyscallError {
    /// Return the errno value of the current error. A syscall can return any value that
    /// fits in an i64, but values between -1 and -4095 are reserved for indicating an
    /// error, so we just convert the error to its negative value. This works similarly
    /// to errno in Linux.
    #[must_use]
    pub fn errno(&self) -> isize {
        -(*self as isize)
    }
}

impl From<BufferError> for SyscallError {
    fn from(e: BufferError) -> Self {
        match e {
            BufferError::NotInUserSpace => Self::BadAddress,
        }
    }
}

impl From<InvalidUserVirtual> for SyscallError {
    fn from(_: InvalidUserVirtual) -> Self {
        Self::BadAddress
    }
}

/// Handle a syscall. This function is called from the syscall interrupt handler, written in
/// assembly and is responsible for dispatching the syscall to the appropriate handler within
/// the kernel.
#[syscall_handler]
#[allow(unused_variables)]
#[allow(clippy::cast_possible_wrap)]
fn syscall(id: usize, a: usize, b: usize, c: usize, d: usize, e: usize) -> isize {
    let result = match Syscall::from(id) {
        Some(Syscall::TaskExit) => task::exit(a),
        Some(Syscall::TaskId) => task::id(),
        Some(Syscall::SerialRead) => serial::read(a, b),
        Some(Syscall::SerialWrite) => serial::write(a, b),

        Some(Syscall::Last) | None => Err(SyscallError::NoSuchSyscall),
    };

    match result {
        Err(error) => error.errno(),
        Ok(value) => value as isize,
    }
}
