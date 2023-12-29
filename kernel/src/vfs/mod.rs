use crate::fs::ramfs;

pub mod fs;
pub mod inode;
pub mod mount;
pub mod name;
pub mod path;

/// Setup the virtual filesystem. It registers all supported filesystems and
/// mounts the root filesystem on the root directory.
pub fn setup() {
    ramfs::register();
    fs::register_root("ramfs");
}
