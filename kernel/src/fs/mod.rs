pub mod ramfs;

/// Register all filesystems supported by the kernel.
pub fn register() {
    ramfs::fs::register();
}
