use crate::device::Device;

pub mod dirent;
pub mod file;
pub mod fs;
pub mod inode;
pub mod mount;
pub mod name;
pub mod path;

/// Setup the virtual filesystem
#[init]
pub fn setup() {
    fs::mount_root("ramfs", Device::None);
}
