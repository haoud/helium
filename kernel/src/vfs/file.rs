use super::{dirent::DirectoryEntry, inode::Inode};
use core::any::Any;

pub struct OpenFile {
    /// The inode opened by this file.
    pub inode: Arc<Inode>,

    /// The operation table for this file.
    pub operation: &'static Operation,

    /// The flags used to open this file.
    pub open_flags: OpenFlags,

    /// The current state of this file. It is stored in a separate structure to
    /// avoid locking the file just to read fields that are never modified, like
    /// the current associated inode.
    pub state: Spinlock<OpenFileState>,

    /// Custom data, freely usable by the filesystem driver.
    pub data: Box<dyn Any + Send + Sync>,
}

/// The state of an open file. It contains informations about the file that
/// can change over time, like the current offset in the file. It is stored
/// in a separate structure to avoid locking the file just to read fields
/// that are never modified.
pub struct OpenFileState {
    /// The current offset in the file.
    pub offset: Offset,
}

/// The operation table for a open file. Depending on the type of the inode
/// opened by the file, the operation table will be different.
pub enum Operation {
    Directory(DirectoryOperation),
    File(FileOperation),
}

bitflags::bitflags! {
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
pub struct Offset(pub i64);

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
pub struct DirectoryOperation {
    /// Reads the directory entry at the given offset.
    ///
    /// # Errors
    /// If the directory entry could not be read, an error is returned, described
    /// by the [`ReaddirError`] enum.
    pub readdir: fn(file: &OpenFile, offset: Offset) -> Result<DirectoryEntry, ReaddirError>,
}

/// The operation table for a file.
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
    pub seek: fn(file: &OpenFile, offset: Offset, whence: Whence) -> Result<Offset, SeekError>,
}

/// The error returned when reading a directory fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReaddirError {}

/// The error returned when reading from a file fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadError {}

/// The error returned when writing to a file fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriteError {}

/// The error returned when seeking into a file fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeekError {}
