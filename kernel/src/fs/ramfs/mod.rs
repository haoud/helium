use crate::vfs;
use alloc::boxed::Box;

pub mod fs;
pub mod mount;

/// Register the ramfs filesystem inside the VFS.
pub fn register() {
    vfs::fs::register(Box::<fs::Filesystem>::default());
}
