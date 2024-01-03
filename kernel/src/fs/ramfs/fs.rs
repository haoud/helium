use crate::{device, vfs};

pub fn read_super(
    device: device::Identifier,
) -> Result<vfs::mount::Super, vfs::fs::ReadSuperError> {
    todo!()
}
