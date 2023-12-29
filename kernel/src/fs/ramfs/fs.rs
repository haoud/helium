use crate::vfs::{self, fs::MountError};

#[derive(Default, Debug, Clone)]
pub struct Filesystem {}

impl Filesystem {}

impl vfs::fs::Filesystem for Filesystem {
    fn mount(&self) -> Result<(), MountError> {
        todo!()
    }

    fn name(&self) -> &str {
        "ramfs"
    }
}
