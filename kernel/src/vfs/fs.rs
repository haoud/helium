use alloc::{boxed::Box, vec::Vec};
use macros::init;
use sync::Spinlock;

/// The list of all registered filesystems.
static FILESYTEMS: Spinlock<Vec<Box<dyn Filesystem>>> = Spinlock::new(Vec::new());

/// The trait that all filesystems must implement.
pub trait Filesystem: Send + Sync {
    /// Mount the filesystem.
    /// 
    /// # Errors
    /// See [`MountError`] for a list of possible errors.
    fn mount(&self) -> Result<(), MountError>;

    /// Returns the name of the filesystem. It must be unique among all
    /// registered filesystems and is used to identify the filesystem.
    fn name(&self) -> &str;
}

/// Register a filesystem.
///
/// # Panics
/// This function will panic if a filesystem with the same name already exists.
pub fn register(fs: Box<dyn Filesystem>) {
    assert!(
        !exists(fs.name()),
        "Filesystem {} already exists",
        fs.name()
    );
    FILESYTEMS.lock().push(fs);
}

/// Unregister a filesystem.
///
/// # Panics
/// This function is not yet implemented and will panic if called.
pub fn unregister(_: Box<dyn Filesystem>) {
    panic!("Unregistering filesystems is not supported yet");
}

/// Check if a filesystem exists.
pub fn exists(name: &str) -> bool {
    FILESYTEMS.lock().iter().any(|fs| fs.name() == name)
}

/// Register the root filesystem.
///
/// This function should only be called once when the kernel is initialising the
/// virtual filesystem.
#[init]
pub fn register_root(_fs: &str) {
    // TODO: Mount the root filesystem.
}

/// Possible errors that can occur while mounting a filesystem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MountError {
    // There is no such filesystem on the specified device.
    NoSuchFilesystem,

    /// The superblock of the filesystem is invalid.
    InvalidSuperblock,

    // An I/O error occurred while mounting the filesystem.
    IOError,
}
