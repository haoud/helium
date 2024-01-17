use super::{dirent::DirectoryEntry, inode::Inode};
use core::any::Any;

#[derive(Debug)]
pub struct File {
    /// The inode opened by this file.
    pub inode: Option<Arc<Inode>>,

    /// The operation table for this file.
    pub operation: Operation,

    /// The flags used to open this file.
    pub open_flags: OpenFlags,

    /// The current state of this file. It is stored in a separate structure to
    /// avoid locking the file just to read fields that are never modified, like
    /// the current associated inode.
    pub state: Spinlock<OpenFileState>,

    /// Custom data, freely usable by the filesystem driver.
    pub data: Box<dyn Any + Send + Sync>,
}

impl File {
    #[must_use]
    pub fn new(info: FileCreateInfo) -> Self {
        let state = OpenFileState { offset: Offset(0) };
        Self {
            inode: info.inode,
            operation: info.operation,
            open_flags: info.open_flags,
            state: Spinlock::new(state),
            data: info.data,
        }
    }

    #[must_use]
    pub fn as_directory(&self) -> Option<&DirectoryOperation> {
        match &self.operation {
            Operation::Directory(d) => Some(d),
            Operation::File(_) => None,
        }
    }

    #[must_use]
    pub fn as_file(&self) -> Option<&FileOperation> {
        match &self.operation {
            Operation::File(f) => Some(f),
            Operation::Directory(_) => None,
        }
    }
}

#[derive(Debug)]
pub struct FileCreateInfo {
    pub inode: Option<Arc<Inode>>,
    pub operation: Operation,
    pub open_flags: OpenFlags,
    pub data: Box<dyn Any + Send + Sync>,
}

/// The state of an open file. It contains informations about the file that
/// can change over time, like the current offset in the file. It is stored
/// in a separate structure to avoid locking the file just to read fields
/// that are never modified.
#[derive(Debug, PartialEq, Eq)]
pub struct OpenFileState {
    /// The current offset in the file.
    pub offset: Offset,
}

/// The operation table for a open file. Depending on the type of the inode
/// opened by the file, the operation table will be different.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operation {
    Directory(&'static DirectoryOperation),
    File(&'static FileOperation),
}

bitflags::bitflags! {
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

/// An offset in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset(pub usize);

/// An file size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Size(pub usize);

/// The seek mode for a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Whence {
    /// Seek from the current offset.
    Current,

    /// Seek from the beginning of the file.
    Start,

    /// Seek from the end of the file.
    End,
}

impl TryFrom<usize> for Whence {
    type Error = ();

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Current),
            1 => Ok(Self::Start),
            2 => Ok(Self::End),
            _ => Err(()),
        }
    }
}

/// The operation table for a directory.
#[derive(Debug, PartialEq, Eq)]
pub struct DirectoryOperation {
    /// Reads the directory entry at the given offset.
    ///
    /// # Errors
    /// If the directory entry could not be read, an error is returned, described
    /// by the [`ReaddirError`] enum.
    pub readdir: fn(file: &File, offset: Offset) -> Result<DirectoryEntry, ReaddirError>,
}

impl DirectoryOperation {
    /// Reads the directory entry at the given offset.
    ///
    /// # Errors
    /// If the directory entry could not be read, an error is returned, described
    /// by the [`ReaddirError`] enum.
    pub fn readdir(&self, file: &File, offset: Offset) -> Result<DirectoryEntry, ReaddirError> {
        (self.readdir)(file, offset)
    }
}

/// The operation table for a file.
#[derive(Debug, PartialEq, Eq)]
pub struct FileOperation {
    /// Writes the given buffer to the file at the given offset, and returns the offset
    /// after the last byte written.
    ///
    /// # Errors
    /// If the buffer could not be written to the file, an error is returned,
    /// described by the [`WriteError`] enum.
    pub write: fn(file: &File, buf: &[u8], offset: Offset) -> Result<usize, WriteError>,

    /// Reads from the file at the given offset into the given buffer, and returns the
    /// offset after the last byte read.
    ///
    /// # Errors
    /// If the buffer could not be read from the file, an error is returned,
    /// described by the [`ReadError`] enum.
    pub read: fn(file: &File, buf: &mut [u8], offset: Offset) -> Result<usize, ReadError>,

    /// Seeks into the file and returns the new offset.
    ///
    /// # Errors
    /// If the seek failed, an error is returned, described by the [`SeekError`] enum.
    pub seek: fn(file: &File, offset: isize, whence: Whence) -> Result<Offset, SeekError>,
}

impl FileOperation {
    /// Writes the given buffer to the file at the given offset, and returns the number
    /// of bytes written.
    ///
    /// # Errors
    /// If the buffer could not be written to the file, an error is returned,
    /// described by the [`WriteError`] enum.
    pub fn write(&self, file: &File, buf: &[u8], offset: Offset) -> Result<usize, WriteError> {
        (self.write)(file, buf, offset)
    }

    /// Reads from the file at the given offset into the given buffer, and returns the
    /// number of bytes read.
    ///
    /// # Errors
    /// If the buffer could not be read from the file, an error is returned,
    /// described by the [`ReadError`] enum.
    pub fn read(&self, file: &File, buf: &mut [u8], offset: Offset) -> Result<usize, ReadError> {
        (self.read)(file, buf, offset)
    }

    /// Seeks into the file and returns the new offset.
    ///
    /// # Errors
    /// If the seek failed, an error is returned, described by the [`SeekError`] enum.
    pub fn seek(&self, file: &File, offset: isize, whence: Whence) -> Result<Offset, SeekError> {
        (self.seek)(file, offset, whence)
    }
}

/// The error returned when reading a directory fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReaddirError {
    /// There is no more entry to read.
    EndOfDirectory,
}

/// The error returned when reading from a file fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadError {
    /// The read operation is not implemented for this file.
    NotImplemented,

    /// The pipe is empty and there are no writers, meaning that the file
    /// will never be written to again and the reader should stop reading.
    BrokenPipe,
}

/// The error returned when writing to a file fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriteError {
    /// The write operation is not implemented for this file.
    NotImplemented,

    /// The pipe is full and there are no readers, meaning that the file
    /// will never be read from again and the writer should stop writing.
    BrokenPipe,
}

/// The error returned when seeking into a file fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeekError {
    /// Overflowing when computing the new offset.
    Overflow,

    /// The opened file is not seekable. This can happen if the
    /// opened file is a pipe or a character device, for example.
    NotSeekable,
}
