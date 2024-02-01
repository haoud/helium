use crate::{
    user::{
        self,
        scheduler::{Scheduler, SCHEDULER},
        string::SyscallString,
    },
    vfs::{self, dentry::Dentry},
};
use alloc::vec;

use super::clock::Timespec;

/// Open a file, specified by `path` with the given `flags`.
///
/// # Errors
/// This function can fail in many ways, and each of them is described by the
/// [`OpenError`] enum.
///
/// # Panics
/// This function panics an inode does not have a corresponding superblock. This
/// should never happen, and is a serious bug in the kernel if it does.
pub fn open(path: usize, flags: usize) -> Result<usize, OpenError> {
    let current_task = SCHEDULER.current_task();
    let root = current_task.root();
    let cwd = current_task.cwd();

    let flags = vfs::file::OpenFlags::from_bits(flags).ok_or(OpenError::InvalidFlag)?;
    let ptr = user::Pointer::<SyscallString>::from_usize(path).ok_or(OpenError::BadAddress)?;
    let path = user::String::from_raw_ptr(&ptr)
        .ok_or(OpenError::BadAddress)?
        .fetch()
        .map_err(|_| OpenError::BadAddress)?;

    let dentry = match vfs::lookup(&path, &root, &cwd) {
        Ok(dentry) => {
            // If the file exists and the `MUST_CREATE` flag is set, we return an error,
            // because the user has specified that the file must be created during the
            // open call.
            if flags.contains(vfs::file::OpenFlags::MUST_CREATE) {
                return Err(OpenError::AlreadyExists);
            }
            dentry
        }
        Err(e) => {
            // The path could not be resolved entirely. This variant contains the
            // last inode that could be resolved and the path that could not be
            // resolved.
            // If only the last component of the path could not be resolved and
            // the `CREATE` flag is set, the kernel will attempt to create a file
            // with the given name in the parent directory
            if let vfs::LookupError::NotFound(parent, path) = e {
                // If the user has not specified the `CREATE` or `MUST_CREATE` flag,
                // we return an error if the file does not exist.
                if !flags.contains(vfs::file::OpenFlags::CREATE)
                    && !flags.contains(vfs::file::OpenFlags::MUST_CREATE)
                {
                    return Err(OpenError::NoSuchFile);
                }

                let name = path.as_name().ok_or(OpenError::NoSuchFile)?.clone();
                Dentry::create_and_fetch_file(parent, name)?
            } else {
                return Err(OpenError::from(e));
            }
        }
    };

    let file = dentry.open(flags)?;
    let id = current_task
        .files()
        .lock()
        .insert(Arc::new(file))
        .ok_or(OpenError::TooManyFilesOpen)?;

    Ok(id.0)
}

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

impl From<vfs::LookupError> for OpenError {
    fn from(error: vfs::LookupError) -> Self {
        match error {
            vfs::LookupError::InvalidPath(_) | vfs::LookupError::NotADirectory => {
                OpenError::InvalidPath
            }
            vfs::LookupError::CorruptedFilesystem => OpenError::UnknownError,
            vfs::LookupError::NotFound(_, _) => OpenError::NoSuchFile,
            vfs::LookupError::IoError => OpenError::IoError,
        }
    }
}

impl From<vfs::dentry::CreateFetchError> for OpenError {
    fn from(error: vfs::dentry::CreateFetchError) -> Self {
        match error {
            vfs::dentry::CreateFetchError::NotADirectory => OpenError::NotADirectory,
            vfs::dentry::CreateFetchError::AlreadyExists => OpenError::AlreadyExists,
            vfs::dentry::CreateFetchError::IoError => OpenError::IoError,
        }
    }
}

impl From<vfs::dentry::OpenError> for OpenError {
    fn from(error: vfs::dentry::OpenError) -> Self {
        match error {}
    }
}

impl From<OpenError> for isize {
    fn from(error: OpenError) -> Self {
        -(error as isize)
    }
}

/// Close a file descriptor.
///
/// # Errors
/// This function return an error if the file descriptor is invalid.
pub fn close(fd: usize) -> Result<usize, CloseError> {
    let current_task = SCHEDULER.current_task();
    current_task
        .files()
        .lock()
        .remove(vfs::fd::Descriptor(fd))
        .ok_or(CloseError::InvalidFileDescriptor)?;

    Ok(0)
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

impl From<CloseError> for isize {
    fn from(error: CloseError) -> Self {
        -(error as isize)
    }
}

/// Read `len` bytes from the file descriptor `fd` into the buffer `buf`.
///
/// # Errors
/// See [`ReadError`] for more details.
///
/// # Panics
/// This function panics if this function try to write more bytes than the user buffer
/// can hold. This is a serious bug in this function if it happens.
pub fn read(fd: usize, buf: usize, len: usize) -> Result<usize, ReadError> {
    let current_task = SCHEDULER.current_task();
    let file = current_task
        .files()
        .lock()
        .get(vfs::fd::Descriptor(fd))
        .ok_or(ReadError::InvalidFileDescriptor)?
        .clone();

    // Check that the file was opened for reading
    if !file.open_flags.contains(vfs::file::OpenFlags::READ) {
        return Err(ReadError::NotReadable);
    }

    let mut read_buffer = vec![0; 256].into_boxed_slice();
    let mut buffer = user::buffer::UserStandardBuffer::new(buf, len)?;

    let mut state = file.state.lock();
    let mut offset = state.offset;
    let mut remaning = len;
    let mut readed = 0;

    while remaning > 0 {
        let bytes_read =
            file.as_file()
                .ok_or(ReadError::NotAFile)?
                .read(&file, &mut read_buffer, offset)?;

        // If there is nothing left to read, we break out of the loop
        if bytes_read == 0 {
            break;
        }

        // Write the readed bytes to the user buffer
        buffer.write_buffered(&read_buffer[..bytes_read]).unwrap();

        // Update the offset, the total number of bytes read and the
        //number of bytes left to read to fill the user buffer
        offset.0 += bytes_read;
        remaning -= bytes_read;
        readed += bytes_read;
    }

    state.offset = offset;
    Ok(readed)
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

    /// The file was not opened with the `Read` flag or the read operation is not
    /// supported for this file
    NotReadable,

    /// The pipe is broken: there is no process with the write end of the pipe open
    /// anymore
    BrokenPipe,

    /// An unknown error occurred
    UnknownError,
}

impl From<user::buffer::BufferError> for ReadError {
    fn from(error: user::buffer::BufferError) -> Self {
        match error {
            user::buffer::BufferError::NotInUserSpace => Self::BadAddress,
        }
    }
}

impl From<vfs::file::ReadError> for ReadError {
    fn from(error: vfs::file::ReadError) -> Self {
        match error {
            vfs::file::ReadError::NotImplemented => Self::NotReadable,
            vfs::file::ReadError::BrokenPipe => Self::BrokenPipe,
        }
    }
}

impl From<ReadError> for isize {
    fn from(error: ReadError) -> Self {
        -(error as isize)
    }
}

/// Write `len` bytes from the buffer `buf` to the file descriptor `fd`.
///
/// # Errors
/// See [`WriteError`] for more details.
///
/// # Panics
/// This function panics if a partial write occurs in the filesystem: this is not yet
/// supported by this syscall.
pub fn write(fd: usize, buf: usize, len: usize) -> Result<usize, WriteError> {
    let current_task = SCHEDULER.current_task();
    let file = current_task
        .files()
        .lock()
        .get(vfs::fd::Descriptor(fd))
        .ok_or(WriteError::InvalidFileDescriptor)?
        .clone();

    // Check that the file was opened for writing
    if !file.open_flags.contains(vfs::file::OpenFlags::WRITE) {
        return Err(WriteError::NotWritable);
    }

    let mut buffer = user::buffer::UserStandardBuffer::new(buf, len)?;

    let mut state = file.state.lock();
    let mut offset = state.offset;
    let mut written = 0;

    while let Some(data) = buffer.read_buffered() {
        let bytes_written = file
            .as_file()
            .ok_or(WriteError::NotAFile)?
            .write(&file, data, offset)?;

        assert!(
            bytes_written == data.len(),
            "Partial writes are not supported"
        );
        offset.0 += bytes_written;
        written += bytes_written;
    }

    state.offset = offset;
    Ok(written)
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

    /// The file was not opened with the `WRITE` flag or the write operation is not
    /// supported for this file
    NotWritable,

    /// The pipe is broken: there is no process with the read end of the pipe open
    /// anymore
    BrokenPipe,

    /// An unknown error occurred
    UnknownError,
}

impl From<user::buffer::BufferError> for WriteError {
    fn from(error: user::buffer::BufferError) -> Self {
        match error {
            user::buffer::BufferError::NotInUserSpace => Self::BadAddress,
        }
    }
}

impl From<vfs::file::WriteError> for WriteError {
    fn from(error: vfs::file::WriteError) -> Self {
        match error {
            vfs::file::WriteError::NotImplemented => Self::NotWritable,
            vfs::file::WriteError::BrokenPipe => Self::BrokenPipe,
        }
    }
}

impl From<WriteError> for isize {
    fn from(error: WriteError) -> Self {
        -(error as isize)
    }
}

/// Repositions the file offset of the open file description associated with the file
/// descriptor `fd` to the argument `offset` according to the directive `whence` as
/// follows:
///  - `Whence::Current`: The offset is set to its current location plus `offset` bytes.
///  - `Whence::End`: The offset is set to the size of the file plus `offset` bytes.
///  - `Whence::Set`: The offset is set to `offset` bytes.
///
/// # Errors
/// See [`SeekError`] for more details.
pub fn seek(fd: usize, offset: usize, whence: usize) -> Result<usize, SeekError> {
    let whence = vfs::file::Whence::try_from(whence).map_err(|_| SeekError::InvalidWhence)?;
    let current_task = SCHEDULER.current_task();
    let file = current_task
        .files()
        .lock()
        .get(vfs::fd::Descriptor(fd))
        .ok_or(SeekError::InvalidFileDescriptor)?
        .clone();

    let mut state = file.state.lock();
    #[allow(clippy::cast_possible_wrap)]
    let offset =
        file.as_file()
            .ok_or(SeekError::NotSeekable)?
            .seek(&file, offset as isize, whence)?;

    state.offset = offset;
    Ok(offset.0)
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

impl From<vfs::file::SeekError> for SeekError {
    fn from(error: vfs::file::SeekError) -> Self {
        match error {
            vfs::file::SeekError::NotSeekable => Self::NotSeekable,
            vfs::file::SeekError::Overflow => Self::Overflow,
        }
    }
}

impl From<SeekError> for isize {
    fn from(error: SeekError) -> Self {
        -(error as isize)
    }
}

/// Get the current working directory of the current process. The path is written to
/// the buffer `buf` and the length of the path is returned.
///
/// # Errors
/// - [`GetCwdError::BadAddress`]: The buffer passed as an argument is invalid
/// - [`GetCwdError::BufferTooSmall`]: The buffer is too small to hold the path
pub fn get_cwd(buf: usize, len: usize) -> Result<usize, GetCwdError> {
    let mut buffer = user::buffer::UserStandardBuffer::new(buf, len)?;
    let current_task = SCHEDULER.current_task();
    let root = current_task.root();
    let cwd = current_task.cwd();

    // Write the path components to the buffer in reverse order
    // (from the last component to the first)
    let path = core::iter::successors(Some(cwd), |dentry| dentry.parent())
        .take_while(|dentry| !Arc::ptr_eq(dentry, &root))
        .map(|dentry| dentry.name().into_inner())
        .collect::<Vec<_>>();

    // Handle the case where the current working directory is the root directory
    if path.is_empty() {
        if len >= 1 {
            _ = buffer.write_buffered("/".as_bytes());
            return Ok(1);
        }
        return Err(GetCwdError::BufferTooSmall);
    }

    // Verify that the buffer is large enough to hold the path
    let path_len = path.iter().fold(0, |acc, name| acc + name.len() + 1);
    if path_len > len {
        return Err(GetCwdError::BufferTooSmall);
    }

    // Write the path components to the buffer in the correct order
    for name in path.iter().rev() {
        _ = buffer.write_buffered("/".as_bytes());
        _ = buffer.write_buffered(name.as_str().as_bytes());
    }

    Ok(path_len)
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

impl From<user::buffer::BufferError> for GetCwdError {
    fn from(error: user::buffer::BufferError) -> Self {
        match error {
            user::buffer::BufferError::NotInUserSpace => Self::BadAddress,
        }
    }
}

impl From<GetCwdError> for isize {
    fn from(error: GetCwdError) -> Self {
        -(error as isize)
    }
}

/// Change the current working directory of the current process to the directory. The
/// directory is specified by its path.
///
/// # Errors
/// See [`ChangeCwdError`] for more details.
pub fn change_cwd(path: usize) -> Result<usize, ChangeCwdError> {
    let ptr = user::Pointer::<SyscallString>::from_usize(path).ok_or(ChangeCwdError::BadAddress)?;
    let path = user::String::from_raw_ptr(&ptr)
        .ok_or(ChangeCwdError::BadAddress)?
        .fetch()?;

    let current_task = SCHEDULER.current_task();
    let root = current_task.root();
    let cwd = current_task.cwd();

    let dentry = vfs::lookup(&path, &root, &cwd)?;
    if dentry.inode().kind != vfs::inode::Kind::Directory {
        return Err(ChangeCwdError::NotADirectory);
    }

    current_task.set_cwd(dentry);
    Ok(0)
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

impl From<vfs::LookupError> for ChangeCwdError {
    fn from(error: vfs::LookupError) -> Self {
        match error {
            vfs::LookupError::NotADirectory => ChangeCwdError::NotADirectory,
            vfs::LookupError::NotFound(_, _) => ChangeCwdError::NoSuchEntry,
            vfs::LookupError::InvalidPath(_) => ChangeCwdError::InvalidPath,
            vfs::LookupError::IoError | vfs::LookupError::CorruptedFilesystem => {
                ChangeCwdError::UnknownError
            }
        }
    }
}

impl From<user::string::FetchError> for ChangeCwdError {
    fn from(e: user::string::FetchError) -> Self {
        match e {
            user::string::FetchError::InvalidMemory => ChangeCwdError::BadAddress,
            user::string::FetchError::StringTooLong => ChangeCwdError::PathTooLong,
            user::string::FetchError::StringNotUtf8 => ChangeCwdError::InvalidUtf8,
        }
    }
}

impl From<ChangeCwdError> for isize {
    fn from(error: ChangeCwdError) -> Self {
        -(error as isize)
    }
}

/// Repositions the file offset of the open file description associated with the file
/// descriptor `fd` to the argument `offset` according to the directive `whence` as
/// follows:
///  - `Whence::Current`: The offset is set to its current location plus `offset` bytes.
///  - `Whence::End`: The offset is set to the size of the file plus `offset` bytes.
///  - `Whence::Set`: The offset is set to `offset` bytes.
///
/// # Errors
/// See [`SeekError`] for more details.
pub fn mkdir(path: usize) -> Result<usize, MkdirError> {
    let ptr = user::Pointer::<SyscallString>::from_usize(path).ok_or(MkdirError::BadAddress)?;
    let path = user::String::from_raw_ptr(&ptr)
        .ok_or(MkdirError::BadAddress)?
        .fetch()?;

    let current_task = SCHEDULER.current_task();
    let root = current_task.root();
    let cwd = current_task.cwd();

    match vfs::lookup(&path, &root, &cwd) {
        Err(vfs::LookupError::NotFound(parent, remaning)) => {
            let name = remaning.as_name().ok_or(MkdirError::NoSuchEntry)?.clone();

            parent
                .inode()
                .as_directory()
                .ok_or(MkdirError::NotADirectory)?
                .mkdir(parent.inode(), name.as_str())?;
        }
        Ok(_) => return Err(MkdirError::AlreadyExists),
        Err(e) => {
            return Err(MkdirError::from(e));
        }
    }

    Ok(0)
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

impl From<vfs::LookupError> for MkdirError {
    fn from(error: vfs::LookupError) -> Self {
        match error {
            vfs::LookupError::NotADirectory => MkdirError::NotADirectory,
            vfs::LookupError::NotFound(_, _) => MkdirError::NoSuchEntry,
            vfs::LookupError::InvalidPath(_) => MkdirError::InvalidPath,
            vfs::LookupError::IoError | vfs::LookupError::CorruptedFilesystem => {
                MkdirError::UnknownError
            }
        }
    }
}

impl From<user::string::FetchError> for MkdirError {
    fn from(e: user::string::FetchError) -> Self {
        match e {
            user::string::FetchError::InvalidMemory => MkdirError::BadAddress,
            user::string::FetchError::StringTooLong => MkdirError::PathTooLong,
            user::string::FetchError::StringNotUtf8 => MkdirError::InvalidUtf8,
        }
    }
}

impl From<vfs::inode::MkdirError> for MkdirError {
    fn from(error: vfs::inode::MkdirError) -> Self {
        match error {
            vfs::inode::MkdirError::AlreadyExists => MkdirError::AlreadyExists,
        }
    }
}

impl From<MkdirError> for isize {
    fn from(error: MkdirError) -> Self {
        -(error as isize)
    }
}

/// Remove an empty directory.
///
/// # Errors
/// See [`RmdirError`] for more details.
///
/// # Panics
/// This function panics if the directory has no parent. This should never happen, and is a
/// serious bug in the kernel if it does.
pub fn rmdir(path: usize) -> Result<usize, RmdirError> {
    let ptr = user::Pointer::<SyscallString>::from_usize(path).ok_or(RmdirError::BadAddress)?;
    let path = user::String::from_raw_ptr(&ptr)
        .ok_or(RmdirError::BadAddress)?
        .fetch()?;

    let current_task = SCHEDULER.current_task();
    let root = current_task.root();
    let cwd = current_task.cwd();

    let dentry = vfs::lookup(&path, &root, &cwd)?;
    let parent = dentry.parent().unwrap();

    parent
        .inode()
        .as_directory()
        .ok_or(RmdirError::NotADirectory)?
        .rmdir(parent.inode(), dentry.name().as_str())?;

    parent.disconnect_child(&dentry.name())?;
    Ok(0)
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

    /// The directory already exists
    AlreadyExists,

    /// The path does not point to a directory
    NotADirectory,

    /// The directory is not empty
    NotEmpty,

    /// An unknown error occurred
    UnknownError,
}

impl From<vfs::LookupError> for RmdirError {
    fn from(error: vfs::LookupError) -> Self {
        match error {
            vfs::LookupError::NotADirectory => RmdirError::NotADirectory,
            vfs::LookupError::NotFound(_, _) => RmdirError::NoSuchEntry,
            vfs::LookupError::InvalidPath(_) => RmdirError::InvalidPath,
            vfs::LookupError::IoError | vfs::LookupError::CorruptedFilesystem => {
                RmdirError::UnknownError
            }
        }
    }
}

impl From<user::string::FetchError> for RmdirError {
    fn from(e: user::string::FetchError) -> Self {
        match e {
            user::string::FetchError::InvalidMemory => RmdirError::BadAddress,
            user::string::FetchError::StringTooLong => RmdirError::PathTooLong,
            user::string::FetchError::StringNotUtf8 => RmdirError::InvalidUtf8,
        }
    }
}

impl From<vfs::inode::RmdirError> for RmdirError {
    fn from(error: vfs::inode::RmdirError) -> Self {
        match error {
            vfs::inode::RmdirError::NotADirectory => RmdirError::NotADirectory,
            vfs::inode::RmdirError::NoSuchEntry => RmdirError::NoSuchEntry,
            vfs::inode::RmdirError::NotEmpty => RmdirError::NotEmpty,
        }
    }
}

impl From<vfs::dentry::DisconnectError> for RmdirError {
    fn from(error: vfs::dentry::DisconnectError) -> Self {
        match error {
            vfs::dentry::DisconnectError::NotFound => RmdirError::NoSuchEntry,
            vfs::dentry::DisconnectError::Busy => RmdirError::NotEmpty,
        }
    }
}

impl From<RmdirError> for isize {
    fn from(error: RmdirError) -> Self {
        -(error as isize)
    }
}

/// Truncate a file to the given length.
///
/// # Errors
/// See [`TruncateError`] for more details.
pub fn truncate(path: usize, len: usize) -> Result<usize, TruncateError> {
    let ptr = user::Pointer::<SyscallString>::from_usize(path).ok_or(TruncateError::BadAddress)?;
    let path = user::String::from_raw_ptr(&ptr)
        .ok_or(TruncateError::BadAddress)?
        .fetch()?;

    let current_task = SCHEDULER.current_task();
    let root = current_task.root();
    let cwd = current_task.cwd();

    let dentry = vfs::lookup(&path, &root, &cwd)?;

    dentry
        .inode()
        .as_file()
        .ok_or(TruncateError::NotAFile)?
        .truncate(dentry.inode(), len)?;

    //dentry.dirtying_inode();
    Ok(0)
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

impl From<vfs::LookupError> for TruncateError {
    fn from(error: vfs::LookupError) -> Self {
        match error {
            vfs::LookupError::NotADirectory => TruncateError::NotADirectory,
            vfs::LookupError::NotFound(_, _) => TruncateError::NoSuchEntry,
            vfs::LookupError::InvalidPath(_) => TruncateError::InvalidPath,
            vfs::LookupError::IoError | vfs::LookupError::CorruptedFilesystem => {
                TruncateError::UnknownError
            }
        }
    }
}

impl From<user::string::FetchError> for TruncateError {
    fn from(e: user::string::FetchError) -> Self {
        match e {
            user::string::FetchError::InvalidMemory => TruncateError::BadAddress,
            user::string::FetchError::StringTooLong => TruncateError::PathTooLong,
            user::string::FetchError::StringNotUtf8 => TruncateError::InvalidUtf8,
        }
    }
}

impl From<vfs::inode::TruncateError> for TruncateError {
    fn from(error: vfs::inode::TruncateError) -> Self {
        match error {}
    }
}

impl From<TruncateError> for isize {
    fn from(error: TruncateError) -> Self {
        -(error as isize)
    }
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
    pub atime: Timespec,

    /// Unix timestamp of the last modification
    pub mtime: Timespec,

    /// Unix timestamp of the last status change
    pub ctime: Timespec,
}

/// Get information about a file.
/// 
/// # Errors
/// See [`StatError`] for more details.
pub fn stat(path: usize, stat: usize) -> Result<usize, StatError> {
    let ptr = user::Pointer::<SyscallString>::from_usize(path).ok_or(StatError::BadAddress)?;
    let path = user::String::from_raw_ptr(&ptr)
        .ok_or(StatError::BadAddress)?
        .fetch()?;

    let ptr = user::Pointer::<Stat>::from_usize(stat).ok_or(StatError::BadAddress)?;

    let current_task = SCHEDULER.current_task();
    let root = current_task.root();
    let cwd = current_task.cwd();

    let dentry = vfs::lookup(&path, &root, &cwd)?;

    let inode = dentry.inode();
    let state = inode.state.lock();
    let stat = Stat {
        dev: 0,  // TODO
        kind: 0, // TODO
        ino: inode.id.0,
        size: state.size as u64,
        nlink: state.links,
        atime: Timespec {
            seconds: state.access_time.0 .0,
            nanoseconds: 0,
        },
        ctime: Timespec {
            seconds: state.access_time.0 .0,
            nanoseconds: 0,
        },
        mtime: Timespec {
            seconds: state.access_time.0 .0,
            nanoseconds: 0,
        },
    };

    unsafe {
        user::Object::write(&ptr, &stat);
    }
    Ok(0)
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

impl From<vfs::LookupError> for StatError {
    fn from(error: vfs::LookupError) -> Self {
        match error {
            vfs::LookupError::NotADirectory => StatError::NotADirectory,
            vfs::LookupError::NotFound(_, _) => StatError::NoSuchEntry,
            vfs::LookupError::InvalidPath(_) => StatError::InvalidPath,
            vfs::LookupError::IoError | vfs::LookupError::CorruptedFilesystem => {
                StatError::UnknownError
            }
        }
    }
}

impl From<user::string::FetchError> for StatError {
    fn from(e: user::string::FetchError) -> Self {
        match e {
            user::string::FetchError::InvalidMemory => StatError::BadAddress,
            user::string::FetchError::StringTooLong => StatError::PathTooLong,
            user::string::FetchError::StringNotUtf8 => StatError::InvalidUtf8,
        }
    }
}

impl From<StatError> for isize {
    fn from(error: StatError) -> Self {
        -(error as isize)
    }
}

#[repr(C)]
pub struct Dirent {
    pub ino: u64,
    pub kind: u16,
    pub name_len: u16,
    pub name: [u8; vfs::name::Name::MAX_LEN],
}

impl Dirent {
    pub const UNKNOWN: u16 = 0;
    pub const REGULAR: u16 = 1;
    pub const DIRECTORY: u16 = 2;
    pub const CHAR_DEVICE: u16 = 3;
    pub const BLOCK_DEVICE: u16 = 4;

    #[must_use]
    pub const fn convert_inode_type(kind: vfs::dirent::Kind) -> u16 {
        match kind {
            vfs::dirent::Kind::File => Self::REGULAR,
            vfs::dirent::Kind::Directory => Self::DIRECTORY,
            vfs::dirent::Kind::CharDevice => Self::CHAR_DEVICE,
            vfs::dirent::Kind::BlockDevice => Self::BLOCK_DEVICE,
        }
    }
}


/// Read a directory entry from the directory descriptor `fd` into the
/// buffer `dirent` from the current position of the directory.
/// 
/// # Errors
/// See [`ReaddirError`] for more details.
pub fn readdir(fd: usize, dirent: usize) -> Result<usize, ReaddirError> {
    let current_task = SCHEDULER.current_task();
    let file = current_task
        .files()
        .lock()
        .get(vfs::fd::Descriptor(fd))
        .ok_or(ReaddirError::InvalidFileDescriptor)?
        .clone();

    let ptr = user::Pointer::<Dirent>::from_usize(dirent).ok_or(ReaddirError::BadAddress)?;

    // Check that the file was opened for reading
    if !file.open_flags.contains(vfs::file::OpenFlags::READ) {
        return Err(ReaddirError::NotReadable);
    }

    let dirent = file
        .as_directory()
        .ok_or(ReaddirError::NotADirectory)?
        .readdir(&file, file.state.lock().offset)?;

    file.state.lock().offset.0 += 1;
    
    unsafe {
        #[allow(clippy::cast_possible_truncation)]
        user::Object::write(&ptr, &Dirent {
            ino: dirent.inode.0,
            kind: Dirent::convert_inode_type(dirent.kind),
            name_len: dirent.name.len() as u16,
            name: dirent.name.as_bytes().try_into().unwrap_or([0; 255]),
        });
    }
    Ok(0)
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

impl From<vfs::file::ReaddirError> for ReaddirError {
    fn from(error: vfs::file::ReaddirError) -> Self {
        match error {
            vfs::file::ReaddirError::EndOfDirectory => ReaddirError::EndOfDirectory,
        }
    }
}

impl From<ReaddirError> for isize {
    fn from(error: ReaddirError) -> Self {
        -(error as isize)
    }
}
