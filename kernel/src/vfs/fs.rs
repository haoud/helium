use super::mount::{MountedSuper, Super};
use alloc::{boxed::Box, sync::Arc, vec::Vec};
use macros::init;
use sync::Spinlock;

/// The list of all registered filesystems.
static FILESYTEMS: Spinlock<Vec<RegisteredFilesystem>> = Spinlock::new(Vec::new());

/// A filesystem that has been registered inside the kernel. This struct is used
/// to keep track of all mounted superblocks for a given filesystem.
pub struct RegisteredFilesystem {
    supers: Vec<Arc<Spinlock<MountedSuper>>>,
    fs: Box<dyn Filesystem>,
}

impl RegisteredFilesystem {
    /// Add the given superblock to the list of mounted superblocks.
    ///
    /// # Panics
    /// This function will panic if the superblock is already in the list of
    /// mounted superblocks.
    pub fn add_super(&mut self, superblock: Arc<Spinlock<MountedSuper>>) {
        assert!(
            !self.supers.iter().any(|s| Arc::ptr_eq(s, &superblock)),
            "Superblock is already mounted"
        );
        self.supers.push(superblock);
    }

    /// Remove the given superblock from the list of mounted superblocks. If the
    /// superblock is not found in the list, this function does nothing.
    pub fn remove_super(&mut self, superblock: &Arc<Spinlock<MountedSuper>>) {
        self.supers.retain(|s| !Arc::ptr_eq(s, superblock));
    }

    /// Returns the list of all mounted superblocks for this filesystem.
    #[must_use]
    pub fn supers(&self) -> &[Arc<Spinlock<MountedSuper>>] {
        &self.supers
    }

    /// Return the inner filesystem.
    #[must_use]
    pub fn inner(&self) -> &dyn Filesystem {
        &*self.fs
    }
}

/// The trait that all filesystems must implement.
pub trait Filesystem: Send {
    /// Mount the filesystem.
    ///
    /// # Errors
    /// See [`MountError`] for a list of possible errors.
    fn mount(&self) -> Result<Box<dyn Super>, MountError>;

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
    FILESYTEMS.lock().push(RegisteredFilesystem {
        supers: Vec::new(),
        fs,
    });
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
    FILESYTEMS.lock().iter().any(|fs| fs.inner().name() == name)
}

/// Register the root filesystem.
///
/// This function should only be called once when the kernel is initialising the
/// virtual filesystem.
#[init]
pub fn register_root(_fs: &str) {
    // TODO: Get the filesystem and mount it.
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
