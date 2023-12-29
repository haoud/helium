use super::inode::{self, Inode};
use alloc::{boxed::Box, sync::Arc};
use bitflags::bitflags;
use hashbrown::HashMap;
use sync::Spinlock;

/// A superblock is a filesystem-specific structure that contains all the
/// information needed to access the filesystem.
pub struct MountedSuper {
    /// A cache of inodes that are currently in use. This cache is not optional and is
    /// not only for performance reasons: it is also needed to ensure that a inode on the
    /// filesystem has only one corresponding inode in memory.
    inodes: Spinlock<HashMap<inode::Identifier, Arc<Inode>>>,

    /// The flags that were used to mount the filesystem.
    mount_flags: MountFlags,

    /// The inner superblock that contains the filesystem-specific operations
    /// and data. It should be unique for each mounted superblock.
    inner: Box<dyn Super>,
}

impl MountedSuper {
    /// Create a new superblock.
    #[must_use]
    pub fn new(inner: Box<dyn Super>, mount_flags: MountFlags) -> Self {
        Self {
            inodes: Spinlock::new(HashMap::new()),
            mount_flags,
            inner,
        }
    }

    /// Returns the flags that were used to mount the filesystem.
    pub fn mount_flags(&self) -> MountFlags {
        self.mount_flags
    }

    /// Returns the inner superblock trait object.
    pub fn inner(&self) -> &dyn Super {
        &*self.inner
    }
}

/// All operations that can be performed on a superblock.
pub trait Super: Send {
    /// Write the given inode to the underlying filesystem.
    ///
    /// # Errors
    /// See [`WriteInodeError`] for a list of possible errors.
    fn write_inode(&self, inode: &Inode) -> Result<(), WriteInodeError>;

    /// Read the inode with the given identifier from the underlying filesystem.
    ///
    /// # Errors
    /// See [`ReadInodeError`] for a list of possible errors.
    fn read_inode(&self, inode: inode::Identifier) -> Result<Inode, ReadInodeError>;

    /// Returns the inode of the root directory.
    ///
    /// # Errors
    /// See [`RootInodeError`] for a list of possible errors.
    fn root_inode(&self) -> Result<Inode, RootInodeError>;

    /// Sync the filesystem with the underlying storage device.
    ///
    /// # Errors
    /// See [`SyncError`] for a list of possible errors.
    fn sync(&self) -> Result<(), SyncError>;

    /// Unmount the filesystem.
    ///
    /// # Errors
    /// See [`UnmountError`] for a list of possible errors.
    fn unmount(&self) -> Result<(), UnmountError>;
}

pub enum WriteInodeError {}

pub enum ReadInodeError {}

pub enum RootInodeError {}

pub enum SyncError {}

pub enum UnmountError {}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MountFlags: u32 {
        /// Mount the filesystem as read-only.
        const READ_ONLY = 1 << 0;
    }
}
