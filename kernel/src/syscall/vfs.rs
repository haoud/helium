use crate::{
    user::{
        self,
        scheduler::{Scheduler, SCHEDULER},
        string::SyscallString,
    },
    vfs::{self, dentry::Dentry},
};
use alloc::vec;

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
