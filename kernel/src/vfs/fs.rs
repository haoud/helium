use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::any::{type_name, Any, TypeId};
use sync::Spinlock;

static FILESYSTEM: Spinlock<Vec<Arc<FileSystemInfo>>> = Spinlock::new(Vec::new());

pub struct FileSystemInfo {
    inner: Box<dyn FileSystem>,
}

impl FileSystemInfo {
    #[must_use]
    pub fn new<T: Default + FileSystem + 'static>() -> Self {
        Self {
            inner: Box::<T>::default(),
        }
    }

    #[must_use]
    pub fn fs(&self) -> &dyn FileSystem {
        &*self.inner
    }

    #[must_use]
    pub fn name(&self) -> &'static str {
        self.inner.name()
    }
}

pub trait FileSystem: Send + Sync + Any {
    fn name(&self) -> &'static str;
}

/// Register a new filesystem. The default filesystem struct is created and*
/// registered in the filesystem list.
///
/// # Panics
/// Panics if the filesystem is already registered.
pub fn register<T: Default + FileSystem + 'static>() {
    assert!(
        !is_registered::<T>(),
        "Filesystem {} is already registered",
        type_name::<T>(),
    );
    FILESYSTEM.lock().push(Arc::new(FileSystemInfo::new::<T>()));
}

/// Check if a filesystem is registered.
pub fn is_registered<T: Default + FileSystem + 'static>() -> bool {
    FILESYSTEM
        .lock()
        .iter()
        .any(|fs| TypeId::of::<T>() == fs.type_id())
}

/// Get a filesystem by name. Returns `None` if the filesystem is not registered.
pub fn get(name: &str) -> Option<Arc<FileSystemInfo>> {
    FILESYSTEM
        .lock()
        .iter()
        .find(|fs| fs.name() == name)
        .cloned()
}

pub fn unregister<T: Default + FileSystem + 'static>() {
    todo!("Unregistering filesystem is not yet supported")
}
