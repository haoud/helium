pub mod ramfs;

/// Register all supported filesystems.
#[init]
pub fn register_all() {
    ramfs::register();
}
