use super::{dirent::DirectoryEntry, inode::Inode};
use core::any::Any;

#[derive(Debug)]
pub struct OpenFile {
    /// The inode opened by this file.
    pub inode: Arc<Inode>,

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

impl OpenFile {
    #[must_use]
    pub fn new(info: OpenFileCreateInfo) -> Self {
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
pub struct OpenFileCreateInfo {
    pub inode: Arc<Inode>,
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
    pub struct OpenFlags: u32 {
        /// The file is opened for reading.
        const READ = 1 << 0;

        /// The file is opened for writing.
        const WRITE = 1 << 1;

        /// The file is created if it does not exist.
        const CREAT = 1 << 2;

        /// The cursor is set at the end of the file when opened.
        const APPEND = 1 << 3;

        /// The file is truncated to 0 length if it exists.
        const TRUNCATE = 1 << 4;
    }
}

/// An offset in a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset(pub u64);

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

/// The operation table for a directory.
#[derive(Debug, PartialEq, Eq)]
pub struct DirectoryOperation {
    /// Reads the directory entry at the given offset.
    ///
    /// # Errors
    /// If the directory entry could not be read, an error is returned, described
    /// by the [`ReaddirError`] enum.
    pub readdir: fn(file: &OpenFile, offset: Offset) -> Result<DirectoryEntry, ReaddirError>,
}

impl DirectoryOperation {
    /// Reads the directory entry at the given offset.
    ///
    /// # Errors
    /// If the directory entry could not be read, an error is returned, described
    /// by the [`ReaddirError`] enum.
    pub fn readdir(&self, file: &OpenFile, offset: Offset) -> Result<DirectoryEntry, ReaddirError> {
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
    pub write: fn(file: &OpenFile, buf: &[u8], offset: Offset) -> Result<Offset, WriteError>,

    /// Reads from the file at the given offset into the given buffer, and returns the
    /// offset after the last byte read.
    ///
    /// # Errors
    /// If the buffer could not be read from the file, an error is returned,
    /// described by the [`ReadError`] enum.
    pub read: fn(file: &OpenFile, buf: &mut [u8], offset: Offset) -> Result<Offset, ReadError>,

    /// Seeks into the file and returns the new offset.
    ///
    /// # Errors
    /// If the seek failed, an error is returned, described by the [`SeekError`] enum.
    pub seek: fn(file: &OpenFile, offset: i64, whence: Whence) -> Result<Offset, SeekError>,
}

impl FileOperation {
    /// Writes the given buffer to the file at the given offset, and returns the offset
    /// after the last byte written.
    ///
    /// # Errors
    /// If the buffer could not be written to the file, an error is returned,
    /// described by the [`WriteError`] enum.
    pub fn write(&self, file: &OpenFile, buf: &[u8], offset: Offset) -> Result<Offset, WriteError> {
        (self.write)(file, buf, offset)
    }

    /// Reads from the file at the given offset into the given buffer, and returns the
    /// offset after the last byte read.
    ///
    /// # Errors
    /// If the buffer could not be read from the file, an error is returned,
    /// described by the [`ReadError`] enum.
    pub fn read(
        &self,
        file: &OpenFile,
        buf: &mut [u8],
        offset: Offset,
    ) -> Result<Offset, ReadError> {
        (self.read)(file, buf, offset)
    }

    /// Seeks into the file and returns the new offset.
    ///
    /// # Errors
    /// If the seek failed, an error is returned, described by the [`SeekError`] enum.
    pub fn seek(&self, file: &OpenFile, offset: i64, whence: Whence) -> Result<Offset, SeekError> {
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
pub enum ReadError {}

/// The error returned when writing to a file fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriteError {}

/// The error returned when seeking into a file fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeekError {
    /// Overflowing when computing the new offset.
    Overflow,
}
