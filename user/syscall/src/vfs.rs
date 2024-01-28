use super::{clock, syscall_return, Errno, Syscall, SyscallString};

pub const O_READ: usize = 1 << 0;
pub const O_WRITE: usize = 1 << 1;
pub const O_CREATE: usize = 1 << 2;
pub const O_TRUNC: usize = 1 << 3;
pub const O_EXCL: usize = 1 << 4;

/// A file descriptor. This is an opaque handle that can be used to refer to
/// an open file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileDescriptor(usize);

pub enum Whence {
    Current(isize),
    Start(isize),
    End(isize),
}

#[repr(C)]
pub struct Stat {
    /// Device ID of device containing file
    pub dev: u64,

    /// Inode number
    pub ino: u64,

    /// Size of the file in bytes
    pub size: u64,

    /// File type
    pub kind: u64,

    /// Number of hard links
    pub nlink: u64,

    /// Unix timestamp of the last access
    pub atime: clock::Timespec,

    /// Unix timestamp of the last modification
    pub mtime: clock::Timespec,

    /// Unix timestamp of the last status change
    pub ctime: clock::Timespec,
}

#[repr(C)]
pub struct Dirent {
    pub ino: u64,
    pub kind: u16,
    pub name_len: u16,
    pub name: [u8; 256],
}

impl Dirent {
    pub const UNKNOWN: u16 = 0;
    pub const REGULAR: u16 = 1;
    pub const DIRECTORY: u16 = 2;
    pub const CHAR_DEVICE: u16 = 3;
    pub const BLOCK_DEVICE: u16 = 4;
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

    /// The pipe is broken: there are no writers and the pipe is empty
    BrokenPipe,

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

    /// The pipe is broken: there are no readers and the pipe is full
    BrokenPipe,

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum GetCwdError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// The buffer passed as an argument is invalid
    BadAddress,

    // The buffer is too small to hold the path
    BufferTooSmall,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for GetCwdError {
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
pub enum ChangeCwdError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// The path passed as an argument is invalid
    BadAddress,

    /// The path is not a valid UTF-8 string
    InvalidUtf8,

    /// The path is invalid
    InvalidPath,

    /// The path is too long
    PathTooLong,

    /// A component of the path is too long
    ComponentTooLong,

    /// The path does not exist
    NoSuchEntry,

    /// The path does not point to a directory
    NotADirectory,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for ChangeCwdError {
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
pub enum MkdirError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// The path passed as an argument
    BadAddress,

    /// The path is not a valid UTF-8 string
    InvalidUtf8,

    /// The path is invalid
    InvalidPath,

    /// The path is too long
    PathTooLong,

    /// A component of the path is too long
    ComponentTooLong,

    /// The path does not exist
    NoSuchEntry,

    /// The directory already exists
    AlreadyExists,

    /// The path does not point to a directory
    NotADirectory,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for MkdirError {
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
pub enum RmdirError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// The path passed as an argument
    BadAddress,

    /// The path is not a valid UTF-8 string
    InvalidUtf8,

    /// The path is invalid
    InvalidPath,

    /// The path is too long
    PathTooLong,

    /// A component of the path is too long
    ComponentTooLong,

    /// The path does not exist
    NoSuchEntry,

    /// The path does not point to a directory
    NotADirectory,

    /// The directory is not empty
    NotEmpty,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for RmdirError {
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
pub enum TruncateError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// The path passed as an argument
    BadAddress,

    /// The path is not a valid UTF-8 string
    InvalidUtf8,

    /// The path is invalid
    InvalidPath,

    /// The path is too long
    PathTooLong,

    /// A component of the path is too long
    ComponentTooLong,

    /// The path does not exist
    NoSuchEntry,

    /// A component of the path prefix is not a directory
    NotADirectory,

    /// The path does not point to a file
    NotAFile,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for TruncateError {
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
pub enum StatError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// The path passed as an argument
    BadAddress,

    /// The path is not a valid UTF-8 string
    InvalidUtf8,

    /// The path is invalid
    InvalidPath,

    /// The path is too long
    PathTooLong,

    /// A component of the path is too long
    ComponentTooLong,

    /// The path does not exist
    NoSuchEntry,

    /// A component of the path prefix is not a directory
    NotADirectory,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for StatError {
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
pub enum ReaddirError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// The file descriptor is invalid
    InvalidFileDescriptor,

    /// The path passed as an argument
    BadAddress,

    /// The descriptor is not a directory
    NotADirectory,

    /// The directory is not readable
    NotReadable,

    /// The directory has no entries remaning
    EndOfDirectory,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for ReaddirError {
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
pub enum UnlinkError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// The path passed as an argument
    BadAddress,

    /// The path is not a valid UTF-8 string
    InvalidUtf8,

    /// The path is invalid
    InvalidPath,

    /// The path is too long
    PathTooLong,

    /// A component of the path is used as a directory, but is not a directory
    ComponentNotADirectory,

    /// A component of the path is too long
    ComponentTooLong,

    /// The path does not exist
    NoSuchEntry,

    /// The path does not point to a directory
    IsADirectory,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for UnlinkError {
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
pub fn open(path: &str, flags: usize, _mode: usize) -> Result<FileDescriptor, OpenError> {
    let str = SyscallString::from(path);
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsOpen as u64,
            in("rsi") &str as *const _ as u64,
            in("rdx") flags as u64,
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

pub fn get_cwd(buffer: &mut [u8]) -> Result<usize, GetCwdError> {
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsGetCwd as u64,
            in("rsi") buffer.as_mut_ptr() as u64,
            in("rdx") buffer.len() as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(GetCwdError::from(errno)),
        Ok(ret) => Ok(ret),
    }
}

pub fn change_cwd(path: &str) -> Result<(), ChangeCwdError> {
    let str = SyscallString::from(path);
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsChangeCwd as u64,
            in("rsi") &str as *const _ as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(ChangeCwdError::from(errno)),
        Ok(_) => Ok(()),
    }
}

pub fn mkdir(path: &str) -> Result<(), MkdirError> {
    let str = SyscallString::from(path);
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsMkdir as u64,
            in("rsi") &str as *const _ as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(MkdirError::from(errno)),
        Ok(_) => Ok(()),
    }
}

pub fn rmdir(path: &str) -> Result<(), RmdirError> {
    let str = SyscallString::from(path);
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsRmdir as u64,
            in("rsi") &str as *const _ as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(RmdirError::from(errno)),
        Ok(_) => Ok(()),
    }
}

pub fn truncate(path: &str, lenght: usize) -> Result<(), TruncateError> {
    let str = SyscallString::from(path);
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsTruncate as u64,
            in("rsi") &str as *const _ as u64,
            in("rdx") lenght as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(TruncateError::from(errno)),
        Ok(_) => Ok(()),
    }
}

pub fn stat(path: &str) -> Result<Stat, TruncateError> {
    let str = SyscallString::from(path);
    let mut stat = Stat {
        dev: 0,
        ino: 0,
        size: 0,
        kind: 0,
        nlink: 0,
        atime: clock::Timespec {
            seconds: 0,
            nanoseconds: 0,
        },
        mtime: clock::Timespec {
            seconds: 0,
            nanoseconds: 0,
        },
        ctime: clock::Timespec {
            seconds: 0,
            nanoseconds: 0,
        },
    };

    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsStat as u64,
            in("rsi") &str as *const _ as u64,
            in("rdx") &mut stat as *mut _ as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(TruncateError::from(errno)),
        Ok(_) => Ok(stat),
    }
}

pub fn readdir(fd: &FileDescriptor) -> Result<Dirent, ReaddirError> {
    let mut dirent = Dirent {
        ino: 0,
        kind: 0,
        name_len: 0,
        name: [0; 256],
    };
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsReaddir as u64,
            in("rsi") fd.0 as u64,
            in("rdx") &mut dirent as *mut _ as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(ReaddirError::from(errno)),
        Ok(_) => Ok(dirent),
    }
}

pub fn unlink(path: &str) -> Result<(), UnlinkError> {
    let str = SyscallString::from(path);
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VfsUnlink as u64,
            in("rsi") &str as *const _ as u64,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(UnlinkError::from(errno)),
        Ok(_) => Ok(()),
    }
}
