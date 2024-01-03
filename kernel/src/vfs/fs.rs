use super::mount::Super;
use crate::device;
use alloc::{sync::Arc, vec::Vec};
use macros::init;
use sync::Spinlock;

/// A list of all the filesystem drivers
static FILESYSTEMS: Spinlock<Vec<Filesystem>> = Spinlock::new(Vec::new());

/// A filesystem driver
pub struct Filesystem {
    /// The name of the filesystem driver
    pub name: &'static str,

    /// The filesystem driver operations
    pub operations: Operations,

    /// A list of all the mounted filesystems
    pub supers: Vec<Arc<Spinlock<Super>>>,
}

/// The operations of a filesystem driver
pub struct Operations {
    /// Reads the superblock of the filesystem on the specified block device
    ///
    /// # Errors
    /// If the superblock could not be read, an error is returned. See [`ReadSuperError`]
    /// for a list of possible errors.
    pub read_super: fn(device: device::Identifier) -> Result<Super, ReadSuperError>,
}

/// A enumeration of possible errors when reading a superblock with [`Operations::read_super`]
pub enum ReadSuperError {
    /// A superblock was present but is invalid
    InvalidSuper,

    /// No superblock was found on the device
    NotFound,

    /// The device could not be read
    IoError,
}

/// Registers a filesystem driver
///
/// # Panics
/// Panics if a filesystem driver with the same name already exists. It should never
/// happen and is a bug in the kernel.
pub fn register(filesystem: Filesystem) {
    assert!(
        !exists(filesystem.name),
        "Filesystem {} already exists",
        filesystem.name
    );
    FILESYSTEMS.lock().push(filesystem);
}

/// Returns whether a filesystem driver with the given name exists or not
pub fn exists(name: &str) -> bool {
    FILESYSTEMS.lock().iter().any(|fs| fs.name == name)
}

/// Finds a filesystem on the block device with the given identifier and
/// mounts as the root filesystem
#[init]
pub fn mount_root(name: &str, device: device::Identifier) {
    todo!();
}
