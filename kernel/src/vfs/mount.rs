use super::inode::{self, Inode};
use crate::device::Device;
use alloc::collections::{BTreeMap, BTreeSet};
use core::any::Any;

/// The superblock of a filesystem. It contains all informations about the
/// filesystem, such as the root inode, the device on which it is mounted, etc.
/// It also contains the operation table for the filesystem.
///
/// When the superblock is dropped, it synchronizes all dirty inodes with the
/// underlying device, and then synchronizes itself with the underlying device
/// to ensure that all informations are up-to-date on the device.
pub struct Super {
    /// The device on which this filesystem is mounted.
    pub device: Device,

    /// The operation table for this filesystem.
    pub operation: &'static Operation,

    /// The root inode of this filesystem.
    pub root: Arc<Inode>,

    /// Custom data, freely usable by the filesystem driver.
    pub data: Box<dyn Any + Send + Sync>,

    /// The list of all used inodes of this filesystem. This improves performances,
    /// but is also required to prevent the kernel from having multiple instances
    /// of the same inode in memory.
    used_inodes: Spinlock<BTreeMap<inode::Identifier, Arc<Inode>>>,

    /// The list of all dirty inodes of this filesystem.
    dirty_inodes: Spinlock<BTreeSet<Arc<Inode>>>,
}

impl Super {
    #[must_use]
    pub fn new(info: SuperCreationInfo) -> Self {
        // Add the root inode to the list of used inodes.
        let mut used_inodes = BTreeMap::new();
        used_inodes.insert(info.root.id, Arc::clone(&info.root));

        Self {
            dirty_inodes: Spinlock::new(BTreeSet::new()),
            used_inodes: Spinlock::new(used_inodes),
            operation: info.operation,
            device: info.device,
            root: info.root,
            data: info.data,
        }
    }

    /// Verifies that the given inode identifier is already cached or not by
    /// the superblock.
    #[must_use]
    pub fn is_cached(&self, id: inode::Identifier) -> bool {
        self.used_inodes.lock().contains_key(&id)
    }

    /// Returns the inode with the given identifier if it is cached by the superblock.
    ///
    /// # Panics
    /// Panics if the inode is already cached by the superblock. It is a bug in the
    /// kernel and should be reported.
    pub fn cache_inode(&mut self, inode: Arc<Inode>) {
        assert!(self.used_inodes.lock().insert(inode.id, inode).is_none());
    }

    /// Get the inode with the given identifier. IT first checks if the inode
    /// is cached by the superblock. If it is not, it reads it from the device
    /// and caches it before returning it.
    ///
    /// # Errors
    /// If the inode is not cached and could not be read from the device, an
    /// error is returned, described by the [`ReadInodeError`] enum.
    pub fn get_inode(&mut self, id: inode::Identifier) -> Result<Arc<Inode>, ReadInodeError> {
        // Check if the inode is cached by the superblock.
        if let Some(inode) = self.used_inodes.lock().get(&id) {
            return Ok(Arc::clone(inode));
        }

        // Read the inode from the device and cache it.
        let inode = Arc::new((self.operation.read_inode)(id)?);
        self.cache_inode(Arc::clone(&inode));
        Ok(inode)
    }

    /// Inserts the given inode in the list of dirty inodes of this filesystem. If
    /// the inode is already in the list, it is not added again and this function
    /// does nothing.
    pub fn make_inode_dirty(&mut self, inode: Arc<Inode>) {
        self.dirty_inodes.lock().insert(inode);
    }

    /// Synchronize all dirty inodes with the underlying device. If an error
    /// occurs, it is logged and the inode is kept in the list of dirty inodes.
    pub fn sync_inodes(&mut self) {
        self.dirty_inodes
            .lock()
            .retain(|inode| match (self.operation.write_inode)(inode) {
                Err(err) => {
                    log::error!("Failed to write inode: {:?}", err);
                    log::error!("Retrying later...");
                    true
                }
                Ok(_) => false,
            });
    }

    /// Sync the superblock with the underlying device. If an error occurs, it is
    /// logged but ignored, because there is nothing we can do about it.
    pub fn sync(&mut self) {
        _ = (self.operation.write_super)(self).map_err(|err| {
            log::error!("Failed to write superblock: {:?}", err);
        });
    }
}

impl Drop for Super {
    fn drop(&mut self) {
        self.sync_inodes();
        self.sync();
    }
}

/// The creation information for a superblock. For more informations about the
/// fields, see the documentation of the [`Super`] structure.
pub struct SuperCreationInfo {
    pub device: Device,
    pub operation: &'static Operation,
    pub root: Arc<Inode>,
    pub data: Box<dyn Any + Send + Sync>,
}

pub struct Operation {
    /// Writes the superblock of this filesystem to the underlying device.
    ///
    /// # Errors
    /// If the superblock could not be written to the device, an error is
    /// returned, described by the [`WriteSuperError`] enum.
    pub write_super: fn(superblock: &Super) -> Result<(), WriteSuperError>,

    /// Writes the given inode to the underlying device.
    ///
    /// # Errors
    /// If the inode could not be written to the device, an error is returned,
    /// described by the [`WriteInodeError`] enum.
    pub write_inode: fn(inode: &Inode) -> Result<(), WriteInodeError>,

    /// Reads the inode with the given identifier from the underlying device.
    ///
    /// # Errors
    /// If the inode could not be read from the device, an error is returned,
    /// described by the [`ReadInodeError`] enum.
    pub read_inode: fn(inode: inode::Identifier) -> Result<Inode, ReadInodeError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriteSuperError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriteInodeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadInodeError {}
