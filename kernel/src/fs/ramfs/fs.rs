use crate::vfs;

pub struct FileSystem;

impl Default for FileSystem {
    fn default() -> Self {
        Self
    }
}

impl vfs::fs::FileSystem for FileSystem {
    fn name(&self) -> &'static str {
        "ramfs"
    }
}

/// Register the ramfs filesystem.
pub fn register() {
    vfs::fs::register::<FileSystem>();
}
