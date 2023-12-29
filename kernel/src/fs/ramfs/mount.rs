use crate::vfs::{
    self,
    mount::{ReadInodeError, RootInodeError, SyncError, UnmountError, WriteInodeError},
};

pub struct SuperBlock {
    
}

impl vfs::mount::Super for SuperBlock {
    fn write_inode(&self, inode: &vfs::inode::Inode) -> Result<(), WriteInodeError> {
        todo!()
    }

    fn read_inode(
        &self,
        inode: vfs::inode::Identifier,
    ) -> Result<vfs::inode::Inode, ReadInodeError> {
        todo!()
    }

    fn root_inode(&self) -> Result<vfs::inode::Inode, RootInodeError> {
        todo!()
    }

    fn sync(&self) -> Result<(), SyncError> {
        todo!()
    }

    fn unmount(&self) -> Result<(), UnmountError> {
        todo!()
    }
}
