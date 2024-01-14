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

pub enum Whence {
    Current(isize),
    Start(isize),
    End(isize),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ReadError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// An invalid file descriptor was passed as an argument
    InvalidFileDescriptor,

    /// The buffer passed as an argument is invalid
    BadAddress,

    /// The file is not a file
    NotAFile,

    /// The file was not opened with the `Read` flag
    NotReadable,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for ReadError {
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
pub enum WriteError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// An invalid file descriptor was passed as an argument
    InvalidFileDescriptor,

    /// The buffer passed as an argument is invalid
    BadAddress,

    /// The file is not a file
    NotAFile,

    /// The file was not opened with the `WRITE` flag
    NotWritable,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for WriteError {
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
pub enum SeekError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// An invalid file descriptor was passed as an argument
    InvalidFileDescriptor,

    /// The file is not seekable
    NotSeekable,

    /// An invalid whence was passed as an argument
    InvalidWhence,

    /// An invalid offset was passed as an argument
    InvalidOffset,

    /// the offset could not be represented by an `isize` and would overflow
    Overflow,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for SeekError {
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

pub fn read(fd: &FileDescriptor, buffer: &mut [u8]) -> Result<usize, ReadError> {
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsRead as u64,
            in("rsi") fd.0 as u64,
            in("rdx") buffer.as_mut_ptr() as u64,
            in("r10") buffer.len() as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(ReadError::from(errno)),
        Ok(ret) => Ok(ret),
    }
}

pub fn write(fd: &FileDescriptor, buffer: &[u8]) -> Result<usize, WriteError> {
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsWrite as u64,
            in("rsi") fd.0 as u64,
            in("rdx") buffer.as_ptr() as u64,
            in("r10") buffer.len() as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(WriteError::from(errno)),
        Ok(ret) => Ok(ret),
    }
}

pub fn seek(fd: &FileDescriptor, whence: Whence) -> Result<usize, SeekError> {
    let (whence, offset) = match whence {
        Whence::Current(offset) => (0, offset),
        Whence::Start(offset) => (1, offset),
        Whence::End(offset) => (2, offset),
    };
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsSeek as u64,
            in("rsi") fd.0 as u64,
            in("rdx") offset as u64,
            in("r10") whence as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(SeekError::from(errno)),
        Ok(ret) => Ok(ret),
    }
}
