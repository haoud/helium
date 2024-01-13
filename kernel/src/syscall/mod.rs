use crate::user::buffer::BufferError;
use addr::user::InvalidUserVirtual;

pub mod mmu;
pub mod serial;
pub mod task;
pub mod video;

/// The type of the return value of a syscall. All syscalls must return a value that fits
/// in an usize. However, some values are reserved for indicating an error: values between
/// -1 and -4095 are reserved for indicating an error (see `SyscallError` for more details).
pub type SyscallValue = usize;

// A struct that contains all the syscall numbers used by the kernel.
#[non_exhaustive]
#[repr(u64)]
pub enum Syscall {
    TaskExit = 0,
    TaskId = 1,
    TaskSleep = 2,
    TaskYield = 3,
    TaskSpawn = 4,
    SerialRead = 5,
    SerialWrite = 6,
    MmuMap = 7,
    MmuUnmap = 8,
    VideoFramebufferInfo = 9,
}

impl Syscall {
    /// Create a new Syscall from a u64. If the u64 is not a valid syscall number, it
    /// returns None.
    #[must_use]
    pub fn from(id: usize) -> Option<Syscall> {
        match id {
            0 => Some(Self::TaskExit),
            1 => Some(Self::TaskId),
            2 => Some(Self::TaskSleep),
            3 => Some(Self::TaskYield),
            4 => Some(Self::TaskSpawn),
            5 => Some(Self::SerialRead),
            6 => Some(Self::SerialWrite),
            7 => Some(Self::MmuMap),
            8 => Some(Self::MmuUnmap),
            9 => Some(Self::VideoFramebufferInfo),
            _ => None,
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
    OutOfMemory = 7,
    AlreadyExists = 8,
    IoError = 9,
    NotADirectory = 10,
    IsADirectory = 11,
    DoesNotExists = 12,
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
        Some(Syscall::TaskSleep) => task::sleep(a),
        Some(Syscall::TaskYield) => task::yields(),
        Some(Syscall::TaskSpawn) => task::spawn(a),
        Some(Syscall::SerialRead) => serial::read(a, b),
        Some(Syscall::SerialWrite) => serial::write(a, b),
        Some(Syscall::MmuMap) => mmu::map(a, b, c, d),
        Some(Syscall::MmuUnmap) => mmu::unmap(a, b),
        Some(Syscall::VideoFramebufferInfo) => video::framebuffer_info(a),
        None => Err(SyscallError::NoSuchSyscall),
    };

    match result {
        Err(error) => error.errno(),
        Ok(value) => value as isize,
    }
}
