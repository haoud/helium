use crate::vfs::{self, fs::MountError, mount::Super};
use super::mount::SuperBlock;
use alloc::boxed::Box;

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Filesystem {}

impl Filesystem {}

impl vfs::fs::Filesystem for Filesystem {
    fn mount(&self) -> Result<Box<dyn Super>, MountError> {
        Ok(Box::new(SuperBlock {}))
    }

    fn name(&self) -> &str {
        "ramfs"
    }
}
