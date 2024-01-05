use super::{InodeDirectory, InodeFile, Superblock};
use crate::{
    device::{Device, Identifier},
    fs::ramfs,
    time::unix::UnixTime,
    vfs::{self, mount::SuperCreationInfo},
};
use alloc::sync::Weak;

pub static FS_OPS: vfs::fs::Operation = vfs::fs::Operation { read_super };

pub static SUPER_OPS: vfs::mount::Operation = vfs::mount::Operation {
    write_super,
    write_inode,
    read_inode,
};

pub static INODE_FILE_OPS: vfs::inode::FileOperation = vfs::inode::FileOperation { truncate };

pub static INODE_DIR_OPS: vfs::inode::DirectoryOperation = vfs::inode::DirectoryOperation {
    mknod,
    create,
    lookup,
    unlink,
    mkdir,
    rmdir,
    link,
    rename,
};

pub static REGULAR_FILE_OPS: vfs::file::FileOperation =
    vfs::file::FileOperation { write, read, seek };

pub static FILE_DIRECTORY_OPS: vfs::file::DirectoryOperation =
    vfs::file::DirectoryOperation { readdir };

/// Read the superblock from the device. Since the ramfs is a memory filesystem,
/// it creates a new superblock in memory, creates a root inode, and returns the
/// VFS superblock.
#[allow(clippy::unnecessary_wraps)]
fn read_super(
    fs: &vfs::fs::Filesystem,
    _: Device,
) -> Result<vfs::mount::Super, vfs::fs::ReadSuperError> {
    let superblock = Superblock::new();
    let root = superblock.get_root_inode();

    // Create the VFS superblock and return it.
    Ok(vfs::mount::Super::new(SuperCreationInfo {
        operation: &SUPER_OPS,
        device: Device::None,
        data: Box::new(Spinlock::new(superblock)),
        root,
    }))
}

/// Write the superblock to the device. Since the ramfs is a memory filesystem,
/// this is a no-op because the superblock is already in memory and is not stored
/// on a device.
///
/// # Errors
/// This function never fails since it is a no-op.
#[allow(clippy::unnecessary_wraps)]
fn write_super(_: &vfs::mount::Super) -> Result<(), vfs::mount::WriteSuperError> {
    Ok(())
}

/// Write the inode to the device. Since the ramfs is a memory filesystem, this
/// is a no-op because the inode is already in memory and is not stored on a
/// device.
///
/// # Errors
/// This function never fails since it is a no-op.
fn write_inode(_: &vfs::inode::Inode) -> Result<(), vfs::mount::WriteInodeError> {
    Ok(())
}

/// Read the inode from the ramfs. If the inode does not exist (the identifier
/// provided is not found in the superblock), an error is returned.
///
/// # Errors
/// If the inode does not exist, an error is returned.
fn read_inode(
    superblock: &vfs::mount::Super,
    id: vfs::inode::Identifier,
) -> Result<Arc<vfs::inode::Inode>, vfs::mount::ReadInodeError> {
    let ramfs_super = superblock
        .data
        .downcast_ref::<Superblock>()
        .expect("Superblock is not a ramfs superblock");

    if let Some(inode) = ramfs_super.inodes.get(&id) {
        Ok(Arc::clone(inode))
    } else {
        Err(vfs::mount::ReadInodeError::DoesNotExist)
    }
}

/// Truncate the file to the given size. If the file is smaller than the given
/// size, it is extended with zeros. If the file is bigger than the given size,
/// it is truncated to the given size.
///
/// # Errors
/// This function never fails.
#[allow(clippy::unnecessary_wraps)]
fn truncate(inode: &vfs::inode::Inode, size: u64) -> Result<u64, vfs::inode::TruncateError> {
    inode
        .data
        .downcast_ref::<Spinlock<InodeFile>>()
        .expect("Inode is not a ramfs inode")
        .lock()
        .content_mut()
        .resize(size as usize, 0);

    Ok(size)
}

fn mknod(
    _inode: &vfs::inode::Inode,
    _name: &str,
    _device: Device,
) -> Result<vfs::inode::Identifier, vfs::inode::CreateError> {
    todo!()
}

/// Create a new file in the directory. If a file with the same name already
/// exists, an error is returned.
///
/// # Errors
/// If a file with the same name already exists, an error is returned.
fn create(
    inode: &vfs::inode::Inode,
    name: &str,
) -> Result<vfs::inode::Identifier, vfs::inode::CreateError> {
    let superblock = inode.superblock.upgrade().unwrap();
    let ramfs_super = superblock
        .data
        .downcast_ref::<Spinlock<Superblock>>()
        .expect("Superblock is not a ramfs superblock");
    let ramfs_inode = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    // Check if a file with the same name already exists.
    if ramfs_inode
        .lock()
        .entries
        .iter()
        .any(|entry| entry.name == name)
    {
        return Err(vfs::inode::CreateError::AlreadyExists);
    }

    // Allocate a new identifier for the file and create the inode.
    let file_id = ramfs::generate_inode_id();
    let file_inode = Arc::new(vfs::inode::Inode::new(
        Weak::clone(&inode.superblock),
        vfs::inode::InodeCreateInfo {
            id: file_id,
            device: Device::None,
            kind: vfs::inode::Kind::File,
            inode_ops: vfs::inode::Operation::File(&INODE_FILE_OPS),
            file_ops: vfs::file::Operation::File(&REGULAR_FILE_OPS),
            state: vfs::inode::InodeState {
                modification_time: UnixTime::now(),
                access_time: UnixTime::now(),
                change_time: UnixTime::now(),
                links: 0,
                size: 0,
            },
            data: Box::new(Spinlock::new(InodeFile::empty())),
        },
    ));

    // Add the inode to the superblock inodes list.
    ramfs_super
        .lock()
        .inodes
        .insert(file_id, Arc::clone(&file_inode));

    // Add the inode to the directory and return its identifier.
    ramfs_inode
        .lock()
        .add_entry(&file_inode, String::from(name))
        .map_err(|_| vfs::inode::CreateError::AlreadyExists)?;
    Ok(file_id)
}

/// Lookup an entry in the directory and return its identifier. If the entry
/// does not exist, an error is returned.
///
/// # Errors
/// If the entry does not exist, an error is returned.
fn lookup(
    inode: &vfs::inode::Inode,
    name: &str,
) -> Result<vfs::inode::Identifier, vfs::inode::LookupError> {
    let ramfs_inode = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    ramfs_inode
        .lock()
        .entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| entry.inode)
        .ok_or(vfs::inode::LookupError::NoSuchEntry)
}

/// Remove an entry from the directory and decrement the links counter of the
/// inode. If the counter reaches 0, the inode is removed from memory.
///
/// # Errors
/// If the entry does not exist or if the caller tries to remove the `.` or `..`
/// entries, an error is returned.
fn unlink(inode: &vfs::inode::Inode, name: &str) -> Result<(), vfs::inode::UnlinkError> {
    let superblock = inode.superblock.upgrade().unwrap();
    let ramfs_super = superblock
        .data
        .downcast_ref::<Spinlock<Superblock>>()
        .expect("Superblock is not a ramfs superblock");
    let ramfs_inode = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    // Find and remove the entry from the directory. If the entry is not
    // found, it return an error.
    let entry = ramfs_inode
        .lock()
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .map(|index| ramfs_inode.lock().entries.remove(index))
        .ok_or(vfs::inode::UnlinkError::NoSuchEntry)?;

    // Fetch the inode and decrement the links counter of the inode.
    // If the counter reaches 0, the inode is removed from the superblock.
    let inode = ramfs_super
        .lock()
        .inodes
        .get(&entry.inode)
        .expect("Dead inode in directory")
        .clone();

    inode.state.lock().links -= 1;
    if inode.state.lock().links == 0 {
        ramfs_super.lock().inodes.remove(&entry.inode);
    }

    Ok(())
}

/// Create a new directory in the directory. If a directory with the same name
/// already exists, an error is returned.
///
/// # Errors
/// If a directory with the same name already exists, an error is returned.
fn mkdir(
    inode: &vfs::inode::Inode,
    name: &str,
) -> Result<vfs::inode::Identifier, vfs::inode::MkdirError> {
    let superblock = inode.superblock.upgrade().unwrap();
    let ramfs_super = superblock
        .data
        .downcast_ref::<Spinlock<Superblock>>()
        .expect("Superblock is not a ramfs superblock");
    let ramfs_inode = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    // Check if a file with the same name already exists.
    if ramfs_inode
        .lock()
        .entries
        .iter()
        .any(|entry| entry.name == name)
    {
        return Err(vfs::inode::MkdirError::AlreadyExists);
    }

    // Allocate a new identifier for the directy and create the inode.
    let directory_id = ramfs::generate_inode_id();
    let mut directory_inode = vfs::inode::Inode::new(
        Weak::clone(&inode.superblock),
        vfs::inode::InodeCreateInfo {
            id: directory_id,
            device: Device::None,
            kind: vfs::inode::Kind::Directory,
            inode_ops: vfs::inode::Operation::Directory(&INODE_DIR_OPS),
            file_ops: vfs::file::Operation::Directory(&FILE_DIRECTORY_OPS),
            state: vfs::inode::InodeState {
                modification_time: UnixTime::now(),
                access_time: UnixTime::now(),
                change_time: UnixTime::now(),
                links: 0,
                size: 0,
            },
            data: Box::new(()),
        },
    );

    // Create the directory data (the `.` and `..` entries)
    directory_inode.data = Box::new(Spinlock::new(InodeDirectory::new(
        &directory_inode,
        inode.id,
    )));
    let directory_inode = Arc::new(directory_inode);

    // Add the inode to the superblock inodes list.
    ramfs_super
        .lock()
        .inodes
        .insert(directory_id, Arc::clone(&directory_inode));

    // Add the inode to the directory and return its identifier.
    ramfs_inode
        .lock()
        .add_entry(&directory_inode, String::from(name))
        .map_err(|_| vfs::inode::MkdirError::AlreadyExists)?;
    Ok(directory_id)
}

/// Remove an directory from the directory and decrement the links counter of the
/// inode. If the counter reaches 0, the inode is removed from memory. The directory
/// must be empty to be removed.
///
/// # Errors
/// If the entry does not exist, if the caller tries to remove the `.` or `..`
/// entries, or if the directory is not empty, an error is returned.
fn rmdir(inode: &vfs::inode::Inode, name: &str) -> Result<(), vfs::inode::RmdirError> {
    let superblock = inode.superblock.upgrade().unwrap();
    let ramfs_super = superblock
        .data
        .downcast_ref::<Spinlock<Superblock>>()
        .expect("Superblock is not a ramfs superblock");
    let ramfs_inode = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    // Find and remove the entry from the directory. If the entry is not
    // found, it return an error.
    let entry = ramfs_inode
        .lock()
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .map(|index| ramfs_inode.lock().entries.remove(index))
        .ok_or(vfs::inode::RmdirError::NoSuchEntry)?;

    // Fetch the inode and verify that the directory is empty.
    let inode = ramfs_super
        .lock()
        .inodes
        .get(&entry.inode)
        .expect("Dead inode in directory")
        .clone();
    let directory_data = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");
    if directory_data.lock().entries.len() > 2 {
        return Err(vfs::inode::RmdirError::NotEmpty);
    }

    // Decrement the links counter of the inode. If the counter reaches 0, the
    // inode is removed from the superblock.
    inode.state.lock().links -= 1;
    if inode.state.lock().links == 0 {
        ramfs_super.lock().inodes.remove(&entry.inode);
    }

    Ok(())
}

/// Create a new hard link to the inode. If a file with the same name already
/// exists, an error is returned.
fn link(
    inode: &vfs::inode::Inode,
    name: &str,
    target: &vfs::inode::Inode,
) -> Result<(), vfs::inode::LinkError> {
    todo!()
}

/// Rename an entry in the directory.
///
/// # Errors
/// If the entry does not exist or if the new name already exists, an error is
/// returned.
fn rename(inode: &vfs::inode::Inode, old: &str, new: &str) -> Result<(), vfs::inode::RenameError> {
    let ramfs_inode = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    // Check if a file with the same name already exists.
    if ramfs_inode
        .lock()
        .entries
        .iter()
        .any(|entry| entry.name == new)
    {
        return Err(vfs::inode::RenameError::AlreadyExists);
    }

    // Find and rename the entry from the directory. If the entry is not
    // found, it return an error.
    ramfs_inode
        .lock()
        .entries
        .iter_mut()
        .find(|entry| entry.name == old)
        .map(|entry| entry.name = String::from(new))
        .ok_or(vfs::inode::RenameError::NoSuchEntry)?;
    Ok(())
}

fn readdir(
    file: &vfs::file::OpenFile,
    offset: vfs::file::Offset,
) -> Result<vfs::dirent::DirectoryEntry, vfs::file::ReaddirError> {
    todo!()
}

fn write(
    file: &vfs::file::OpenFile,
    buf: &[u8],
    offset: vfs::file::Offset,
) -> Result<vfs::file::Offset, vfs::file::WriteError> {
    todo!()
}

fn read(
    file: &vfs::file::OpenFile,
    buf: &mut [u8],
    offset: vfs::file::Offset,
) -> Result<vfs::file::Offset, vfs::file::ReadError> {
    todo!()
}

fn seek(
    file: &vfs::file::OpenFile,
    offset: vfs::file::Offset,
    whence: vfs::file::Whence,
) -> Result<vfs::file::Offset, vfs::file::SeekError> {
    todo!()
}
