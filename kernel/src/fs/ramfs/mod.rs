use alloc::boxed::Box;
use crate::vfs;

pub mod fs;

/// Register the ramfs filesystem inside the VFS.
pub fn register() {
    vfs::fs::register(Box::<fs::Filesystem>::default());
}
