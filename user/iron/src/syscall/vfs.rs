use super::{syscall_return, Errno, Syscall, SyscallString};

/// A file descriptor. This is an opaque handle that can be used to refer to
/// an open file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileDescriptor(usize);

bitflags::bitflags! {
    /// Flags that can be passed to the `open` syscall. These flags can be
    /// combined using the `|` operator, but some flags are mutually exclusive.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct OpenFlags: usize {
        /// The file is opened for reading.
        const READ = 1 << 0;

        /// The file is opened for writing.
        const WRITE = 1 << 1;

        /// The file is created if it does not exist. If the file exists,
        /// it is simply opened.
        const CREATE = 1 << 2;

        /// The file is truncated to 0 length if it exists.
        const TRUNCATE = 1 << 3;

        /// The file must be created during the open call. If the file already
        /// exists, the call will fail.
        const MUST_CREATE = 1 << 4;
    }
}

/// Errors that can occur during the `open` syscall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum OpenError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// An invalid address was passed as an argument
    BadAddress,

    /// The path is invalid
    InvalidPath,

    /// An invalid flag or flags combination was passed to the syscall
    InvalidFlag,

    /// The file does not exist
    NoSuchFile,

    // One of the components of the path is not a directory
    NotADirectory,

    /// The path does not point to a file
    NotAFile,

    /// An I/O error occurred
    IoError,

    /// The file already exists
    AlreadyExists,

    /// The kernel ran out of memory while spawning the task
    OutOfMemory,

    /// The process has too many files open and cannot open any more
    TooManyFilesOpen,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for OpenError {
    fn from(error: Errno) -> Self {
        if error.code() > -(Self::UnknownError as isize) {
            unsafe { core::mem::transmute(error) }
        } else {
            Self::UnknownError
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum CloseError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// An invalid file descriptor was passed as an argument
    InvalidFileDescriptor,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for CloseError {
    fn from(error: Errno) -> Self {
        if error.code() > -(Self::UnknownError as isize) {
            unsafe { core::mem::transmute(error) }
        } else {
            Self::UnknownError
        }
    }
}

/// Open a file and return a file descriptor that can be used to refer to it.
///
/// # Errors
/// See `OpenError` for a list of possible errors.
pub fn open(path: &str, flags: OpenFlags) -> Result<FileDescriptor, OpenError> {
    let str = SyscallString::from(path);
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsOpen as u64,
            in("rsi") &str as *const _ as u64,
            in("rdx") flags.bits() as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(OpenError::from(errno)),
        Ok(ret) => Ok(FileDescriptor(ret)),
    }
}

pub fn close(fd: FileDescriptor) -> Result<(), CloseError> {
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsClose as u64,
            in("rsi") fd.0 as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(CloseError::from(errno)),
        Ok(_) => Ok(()),
    }
}
