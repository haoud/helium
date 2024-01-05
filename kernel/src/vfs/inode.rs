use super::{dirent, file, inode, mount::Super};
use crate::{
    device::{self, Device},
    time::unix::UnixTime,
};
use alloc::sync::Weak;
use core::any::Any;

/// The root inode of the filesystem. It is initialized when the filesystem
/// is mounted, and is used by the VFS to access the filesystem.
pub static ROOT: Once<Arc<Inode>> = Once::new();

/// The identifier of an inode. It must be unique among all inodes of the
/// filesystem and is used by the superblock to cache inodes and retrieve
/// them later without having to search them in the filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identifier(pub u64);

///
/// The inode structure implements `PartialEq`, `Eq`, `PartialOrd` and `Ord` to
/// allow the use of the container types provided by the standard library.
/// The comparison is only done by comparing the inode identifiers, since the VFS
/// assume that all inodes of a filesystem have a unique identifier.
pub struct Inode {
    /// The identifier of this inode. It is unique among all inodes of the
    /// filesystem.
    pub id: Identifier,

    /// The device on which this inode is stored.
    pub device: Device,

    /// The type of this inode. It is necessary because a directory and a file
    /// may have different data structures.
    pub kind: Kind,

    /// The operation table for this inode. It may differs depending on the
    /// type of the inode. For example, a directory and a file have different
    /// operations because they must be handled differently.
    pub inode_ops: inode::Operation,

    /// The operation table for this inode if opened as a file. It is necessary
    /// because block or character devices, when opened on a filesystem, have
    /// a different file operation table because they does not interact with
    /// the filesystem.
    pub file_ops: file::Operation,

    /// The state of this inode. It contains informations about the inode that
    /// can change over time, like the last time it has been modified. It is
    /// stored in a separate structure to avoid locking the inode just to read
    /// fields that are never modified, like the identifier or the inode type.
    pub state: Spinlock<InodeState>,

    /// Custom data associated with this inode. It is used by the filesystem
    /// to store informations about the inode that are not stored in the inode
    /// itself, inclusing filesystem-specific informations.
    pub data: Box<dyn Any + Send + Sync>,

    /// The superblock of this inode.
    pub superblock: Weak<Super>,
}

impl Inode {
    #[must_use]
    pub fn new(superblock: Weak<Super>, info: InodeCreateInfo) -> Self {
        Self {
            state: Spinlock::new(info.state),
            inode_ops: info.inode_ops,
            file_ops: info.file_ops,
            device: info.device,
            data: info.data,
            kind: info.kind,
            id: info.id,
            superblock,
        }
    }

    /// Returns the kind of this inode, but using the dirent Kind structure
    /// instead of the inode Kind structure that contains more informations
    /// and can be annoying to use.
    #[must_use]
    pub fn dirent_kind(&self) -> dirent::Kind {
        dirent::Kind::from(self.kind)
    }
}

impl Ord for Inode {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialEq for Inode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for Inode {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Inode {}

impl Drop for Inode {
    fn drop(&mut self) {
        // There is no need to synchronize the inode with the underlying device
        // if it is dirty because the drop is called when the inode is entirely
        // removed from the memory. Since the superblock maintains a reference
        // to the inode if it is dirty, it will be synchronized later and will
        // be dropped only when it is clean.
    }
}

pub struct InodeState {
    /// The last time the inode data has been modified.
    pub modification_time: UnixTime,

    /// The last time this inode has been accessed.
    pub access_time: UnixTime,

    /// The last time this inode state has been changed.
    pub change_time: UnixTime,

    /// The number of hard links to this inode.
    pub links: u64,

    /// The size of this inode, in bytes.
    pub size: u64,
}

/// The type of an inode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    BlockDevice(device::Identifier),
    CharDevice(device::Identifier),
    Directory,
    File,
}

/// The creation information for an inode. For more informations about the
/// fields, see the documentation of the [`Inode`] structure.
pub struct InodeCreateInfo {
    pub id: Identifier,
    pub device: Device,
    pub kind: Kind,
    pub inode_ops: inode::Operation,
    pub file_ops: file::Operation,
    pub state: InodeState,
    pub data: Box<dyn Any + Send + Sync>,
}

pub enum Operation {
    Directory(&'static DirectoryOperation),
    File(&'static FileOperation),
}

impl Operation {
    /// Returns the operation table for this inode if it is a file inode,
    /// or `None` if it is a directory inode.
    #[must_use]
    pub fn as_file(&self) -> Option<&FileOperation> {
        match self {
            Self::Directory(_) => None,
            Self::File(file) => Some(file),
        }
    }

    /// Returns the operation table for this inode if it is a directory inode,
    /// or `None` if it is a file inode.
    #[must_use]
    pub fn as_directory(&self) -> Option<&DirectoryOperation> {
        match self {
            Self::Directory(dir) => Some(dir),
            Self::File(_) => None,
        }
    }
}

pub struct DirectoryOperation {
    /// Creates a new device inode with the given name and device identifier in the given
    /// directory, and returns the identifier of the new inode.
    ///
    /// # Errors
    /// If the inode could not be created, an error is returned, described by
    /// the [`MknodError`] enum.
    pub mknod: fn(inode: &Inode, name: &str, device: Device) -> Result<Identifier, CreateError>,

    /// Creates a new regular file with the given name in the given directory, and returns
    /// the identifier of the new inode.
    ///
    /// # Errors
    /// If the inode could not be created, an error is returned, described by
    /// the [`CreateError`] enum.
    pub create: fn(inode: &Inode, name: &str) -> Result<Identifier, CreateError>,

    /// Looks up the inode with the given name in the given directory, and returns its
    /// identifier.
    ///
    /// # Errors
    /// If the inode could not be found, an error is returned, described by
    /// the [`LookupError`] enum.
    pub lookup: fn(inode: &Inode, name: &str) -> Result<Identifier, LookupError>,

    /// Removes the inode with the given name from the given directory.
    ///
    /// # Errors
    /// If the inode could not be removed, an error is returned, described by
    /// the [`LookupError`] enum.
    pub unlink: fn(inode: &Inode, name: &str) -> Result<(), UnlinkError>,

    /// Creates a new directory with the given name in the given directory, and returns
    /// the identifier of the new inode.
    ///
    /// # Errors
    /// If the inode could not be created, an error is returned, described by
    /// the [`MkdirError`] enum.
    pub mkdir: fn(inode: &Inode, name: &str) -> Result<Identifier, MkdirError>,

    /// Removes the directory with the given name from the given directory.
    ///
    /// # Errors
    /// If the directory could not be removed, an error is returned, described by
    /// the [`RmdirError`] enum.
    pub rmdir: fn(inode: &Inode, name: &str) -> Result<(), RmdirError>,

    /// Creates a new hard link with the given name in the given directory, pointing to the
    /// given inode.
    ///
    /// # Errors
    /// If the link could not be created, an error is returned, described by
    /// the [`LinkError`] enum.
    pub link: fn(inode: &Inode, name: &str, target: &Inode) -> Result<(), LinkError>,

    /// Renames the inode with the given name in the given directory to the given name.
    ///
    /// # Errors
    /// If the inode could not be renamed, an error is returned, described by
    /// the [`RenameError`] enum.
    pub rename: fn(inode: &Inode, old: &str, new: &str) -> Result<(), RenameError>,
}

pub struct FileOperation {
    /// Truncates the inode data to the given size. If the size is greater than the current
    /// size of the inode, the inode data is extended and the new bytes are filled with zeros.
    /// If the size is less than the current size of the inode, the inode data is truncated.
    ///
    /// # Errors
    /// If the inode could not be truncated, an error is returned, described by
    /// the [`TruncateError`] enum.
    pub truncate: fn(inode: &Inode, size: u64) -> Result<u64, TruncateError>,
}

/// The error returned when an inode could not be truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TruncateError {}

/// The error returned when an file inode could not be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreateError {
    /// An entry with the same name already exists in the directory.
    AlreadyExists,
}

/// The error returned when an inode could not be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MknodError {}

/// The error returned when an inode could not be found i na directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LookupError {
    /// The entry does not exist in the directory.
    NoSuchEntry,
}

/// The error returned when a directory could not be removed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RmdirError {
    /// The entry does not exist in the directory.
    NoSuchEntry,

    /// The entry is not a directory and cannot be removed with the `rmdir` but
    /// with the `unlink` operation instead.
    NotADirectory,

    /// The directory is not empty.
    NotEmpty,
}

/// The error returned when a directory could not be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MkdirError {
    /// An entry with the same name already exists in the directory.
    AlreadyExists,
}

/// The error returned when an inode could not be renamed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenameError {
    /// The entry does not exist in the directory.
    NoSuchEntry,

    /// An entry with the same name already exists in the directory.
    AlreadyExists,
}

/// The error returned when a link could not be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LinkError {}

/// The error returned when an inode could not be unlinked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnlinkError {
    /// The entry is reserved for the filesystem usage and cannot be removed.
    /// This error is returned when trying to remove the `.` and `..` entries
    /// of a directory in some filesystems.
    ReservedEntry,

    /// The entry is a directory that must be removed with the `rmdir` operation
    /// and not the `unlink` operation.
    IsADirectory,

    /// The entry does not exist in the directory.
    NoSuchEntry,
}
