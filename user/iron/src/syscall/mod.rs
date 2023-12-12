//! ## How to make a syscall in assembly (x86_64)
//!  - Put the syscall number in rax
//!  - Put the arguments (if any) respectively in rsi, rdx, r10, r8, r9. If fewer than 5
//!    arguments are used, the remaining registers will be ignored and unchanged.
//!  - Execute the syscall instruction
//!  - The return value is in rax, others registers are preserved during the syscall
//!
//!
//! Example:
//! ```rust
//! let result;
//! unsafe {
//!     core::arch::asm!(
//!       "syscall",
//!         in("rax") 0,    // Syscall number
//!         in("rsi") 0,    // Argument 1
//!         in("rdx") 0,    // Argument 2
//!         in("r10") 0,    // Argument 3
//!         in("r8") 0,     // Argument 4
//!         in("r9") 0,     // Argument 5
//!         lateout("rax") result);
//! }
//! ```

pub mod mmu;
pub mod serial;
pub mod task;

#[repr(u64)]
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Errno {
    /// The errno code is unknown.
    Unknown = 0,

    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// One (or more) argument passed to the syscall is invalid.
    InvalidArgument = 2,

    /// The task with the given id does not exist.
    TaskNotFound = 3,

    /// The task is already in use.
    TaskInUse = 4,

    /// The address passed to the syscall is invalid.
    BadAddress = 5,

    /// The requestred syscall exist but is not implemented.
    NotImplemented = 6,

    /// The kernel ran out of memory while handling the syscall.
    OutOfMemory = 7,

    /// The resource already exists.
    AlreadyExists = 8,
}

impl From<usize> for Errno {
    fn from(value: usize) -> Self {
        match value {
            1 => Errno::NoSuchSyscall,
            2 => Errno::InvalidArgument,
            3 => Errno::TaskNotFound,
            4 => Errno::TaskInUse,
            5 => Errno::BadAddress,
            6 => Errno::NotImplemented,
            7 => Errno::OutOfMemory,
            8 => Errno::AlreadyExists,
            _ => Errno::Unknown,
        }
    }
}

impl Errno {
    /// Create an `Errno` from a syscall return value. If the return value is not an error,
    /// this function will return `None`.
    #[must_use]
    pub fn from_syscall_return(register: usize) -> Option<Self> {
        match Self::syscall_error(register) {
            true => Some(Self::from((register as isize).unsigned_abs())),
            false => None,
        }
    }

    /// Verify if the syscall return value is an error. An syscall is allowed to return
    /// any value excepted value greater than `usize::MAX - 4096` that are reserved for
    /// indicating an error.
    #[must_use]
    pub const fn syscall_error(register: usize) -> bool {
        register > (usize::MAX - 4096)
    }
}
