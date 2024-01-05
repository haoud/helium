use super::{inode, mount::Super};
use crate::device::Device;
use core::any::Any;

/// The list of all registered filesystems.
static FILESYSTEMS: Spinlock<Vec<Arc<Filesystem>>> = Spinlock::new(Vec::new());

pub struct Filesystem {
    /// The name of this filesystem. It must be unique among all filesystems.
    name: &'static str,

    /// The operation table for this filesystem.
    operation: &'static Operation,

    /// The list of all mounted filesystems of this type.
    supers: Spinlock<Vec<Arc<Super>>>,

    /// The filesystem-specific data.
    data: Box<dyn Any + Send + Sync>,
}

impl Filesystem {
    #[must_use]
    pub fn new(
        name: &'static str,
        operation: &'static Operation,
        data: Box<dyn Any + Send + Sync>,
    ) -> Self {
        Self {
            supers: Spinlock::new(Vec::new()),
            operation,
            name,
            data,
        }
    }

    /// Reads the superblock of this filesystem from the given device and
    /// return a VFS superblock.
    ///
    /// # Errors
    /// If the superblock could not be read from the device, an error is
    /// returned, described by the [`ReadSuperError`] enum.
    pub fn read_super(&self, device: Device) -> Result<Super, ReadSuperError> {
        (self.operation.read_super)(self, device)
    }
}

/// The operation table for a filesystem.
pub struct Operation {
    /// Reads the superblock of this filesystem from the given device and
    /// return a VFS superblock.
    ///
    /// # Errors
    /// If the superblock could not be read from the device, an error is
    /// returned, described by the [`ReadSuperError`] enum.
    pub read_super: fn(fs: &Filesystem, device: Device) -> Result<Super, ReadSuperError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReadSuperError {
    /// The filesystem was found on the device, but it is corrupted.
    CorruptedFileSystem,

    /// The device does not contain a filesystem of this type.
    InvalidFileSystem,

    /// The function is not implemented for this filesystem.
    NotImplemented,

    /// The device does not exist.
    InvalidDevice,

    /// An I/O error occurred while reading from the device.
    IoError,
}

/// Registers a filesystem
///
/// # Panics
/// Panics if a filesystem with the same name already exists.
pub fn register(fs: Filesystem) {
    assert!(!exists(fs.name), "Filesystem {} already exists", fs.name);
    FILESYSTEMS.lock().push(Arc::new(fs));
}

/// Verifies that a filesystem with the given name exists or not.
pub fn exists(name: &str) -> bool {
    FILESYSTEMS.lock().iter().any(|fs| fs.name == name)
}

/// Mount the filesystem with the given name on the given device and initialize
/// the root inode.
#[init]
pub fn mount_root(name: &str, device: Device) {
    let fs = FILESYSTEMS
        .lock()
        .iter()
        .find(|fs| fs.name == name)
        .expect("Filesystem not found")
        .clone();
    let superblock = fs.read_super(device).expect("Failed to read superblock");

    // Initialize the root inode and push the superblock to the list
    // of mounted filesystems
    inode::ROOT.call_once(|| Arc::clone(&superblock.root));
    fs.supers.lock().push(Arc::new(superblock));
}
