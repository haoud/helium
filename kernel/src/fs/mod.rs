pub mod ramfs;

/// Register all supported filesystems.
pub fn register_all() {
    ramfs::register();
}
