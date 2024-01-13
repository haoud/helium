use super::{generate_inode_id, InodeDirectory, InodeFile, Superblock};
use crate::{
    device::Device,
    fs::ramfs,
    time::unix::UnixTime,
    vfs::{self, mount::SuperCreationInfo},
};
use alloc::sync::Weak;

/// Operations that can be performed on the filesystem.
pub static FS_OPS: vfs::fs::Operation = vfs::fs::Operation { read_super };

/// Operations that can be performed on the superblock.
pub static SUPER_OPS: vfs::mount::Operation = vfs::mount::Operation {
    write_super,
    write_inode,
    read_inode,
};

/// Operations that can be performed on a directory inode.
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

/// Operations that can be performed on a file inode.
pub static INODE_FILE_OPS: vfs::inode::FileOperation = vfs::inode::FileOperation { truncate };

/// Operations that can be performed on a opened regular file.
pub static REGULAR_FILE_OPS: vfs::file::FileOperation =
    vfs::file::FileOperation { write, read, seek };

/// Operations that can be performed on a opened directory.
pub static FILE_DIRECTORY_OPS: vfs::file::DirectoryOperation =
    vfs::file::DirectoryOperation { readdir };

/// Read the superblock from the device. Since the ramfs is a memory filesystem,
/// it creates a new superblock in memory, creates a root inode, and returns the
/// VFS superblock.
///
/// # Errors
/// This function never fails.
#[allow(clippy::unnecessary_wraps)]
fn read_super(
    _: &vfs::fs::Filesystem,
    _: Device,
) -> Result<Arc<vfs::mount::Super>, vfs::fs::ReadSuperError> {
    let ramfs_superblock = Superblock::default();
    let root_id = ramfs_superblock.get_root_inode_id();
    let vfs_superblock = Arc::new(vfs::mount::Super::new(SuperCreationInfo {
        operation: &SUPER_OPS,
        device: Device::None,
        data: Box::new(Spinlock::new(ramfs_superblock)),
        root: root_id,
    }));

    // Create the root inode and add it to the ramfs super
    let mut root = vfs::inode::Inode::new(
        Arc::downgrade(&vfs_superblock),
        vfs::inode::InodeCreateInfo {
            id: generate_inode_id(),
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

    // Create the root directory data (the `.` and `..` entries) and add it to
    // the root inode before returning the superblock.
    root.data = Box::new(Spinlock::new(InodeDirectory::new(&root, root_id)));
    vfs_superblock
        .data()
        .downcast_ref::<Spinlock<Superblock>>()
        .unwrap()
        .lock()
        .inodes
        .insert(root_id, Arc::new(root));
    Ok(vfs_superblock)
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
#[allow(clippy::unnecessary_wraps)]
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
        .data()
        .downcast_ref::<Spinlock<Superblock>>()
        .expect("Superblock is not a ramfs superblock");

    if let Some(inode) = ramfs_super.lock().inodes.get(&id) {
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
fn truncate(inode: &vfs::inode::Inode, size: usize) -> Result<usize, vfs::inode::TruncateError> {
    inode.state.lock().size = size;
    inode
        .data
        .downcast_ref::<Spinlock<InodeFile>>()
        .expect("Inode is not a ramfs inode")
        .lock()
        .content_mut()
        .resize(size, 0);

    Ok(size)
}

fn mknod(
    _inode: &vfs::inode::Inode,
    _name: &str,
    _device: Device,
) -> Result<vfs::inode::Identifier, vfs::inode::CreateError> {
    todo!()
}

/// Create a new file in the directory.
///
/// # Errors
/// If a file with the same name already exists, an error is returned.
fn create(
    inode: &vfs::inode::Inode,
    name: &str,
) -> Result<vfs::inode::Identifier, vfs::inode::CreateError> {
    let superblock = inode.superblock.upgrade().unwrap();
    let ramfs_super = superblock
        .data()
        .downcast_ref::<Spinlock<Superblock>>()
        .expect("Superblock is not a ramfs superblock");
    let ramfs_inode = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    // Check if a file with the same name already exists.
    let mut locked_dir = ramfs_inode.lock();
    if locked_dir.entries.iter().any(|entry| entry.name == name) {
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
    locked_dir.add_entry(&file_inode, String::from(name));
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
/// If the entry does not exist
fn unlink(inode: &vfs::inode::Inode, name: &str) -> Result<(), vfs::inode::UnlinkError> {
    let superblock = inode.superblock.upgrade().unwrap();
    let ramfs_super = superblock
        .data()
        .downcast_ref::<Spinlock<Superblock>>()
        .expect("Superblock is not a ramfs superblock");
    let ramfs_inode = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    // Find the entry in the directory.
    let mut locked_dir = ramfs_inode.lock();
    let index = locked_dir
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .ok_or(vfs::inode::UnlinkError::NoSuchEntry)?;

    // If the entry is a directory, return an error.
    if locked_dir.entries[index].kind == vfs::dirent::Kind::Directory {
        return Err(vfs::inode::UnlinkError::IsADirectory);
    }

    // Fetch the inode and decrement the links counter of the inode.
    // If the counter reaches 0, the inode is removed from the superblock.
    let entry = locked_dir.entries.remove(index);
    let inode = ramfs_super
        .lock()
        .inodes
        .get(&entry.inode)
        .expect("Dead inode in directory")
        .clone();

    if inode.state.lock().unlinked() == 0 {
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
        .data()
        .downcast_ref::<Spinlock<Superblock>>()
        .expect("Superblock is not a ramfs superblock");
    let ramfs_inode = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    // Check if a file with the same name already exists.
    let mut locked_dir = ramfs_inode.lock();
    if locked_dir.entries.iter().any(|entry| entry.name == name) {
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
    locked_dir.add_entry(&directory_inode, String::from(name));
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
        .data()
        .downcast_ref::<Spinlock<Superblock>>()
        .expect("Superblock is not a ramfs superblock");
    let ramfs_inode = inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    // Check if the caller tries to remove the `.` or `..` entries. If so, return
    // an error by indicating that the entry does not exist (reserved by the
    // filesystem implementation).
    if name == "." || name == ".." {
        return Err(vfs::inode::RmdirError::NoSuchEntry);
    }

    // Find and remove the entry from the directory. If the entry is not
    // found, it return an error.
    let mut locked_dir = ramfs_inode.lock();
    let index = locked_dir
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .ok_or(vfs::inode::RmdirError::NoSuchEntry)?;

    // If the entry is not a directory, return an error.
    if locked_dir.entries[index].kind != vfs::dirent::Kind::Directory {
        return Err(vfs::inode::RmdirError::NotADirectory);
    }

    // Fetch the inode and verify that the directory is empty.
    let entry = locked_dir.entries.remove(index);
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
    if inode.state.lock().unlinked() == 0 {
        ramfs_super.lock().inodes.remove(&entry.inode);
    }
    Ok(())
}

/// Create a new hard link to the inode. If a file with the same name already
/// exists, an error is returned.
fn link(
    _inode: &vfs::inode::Inode,
    _name: &str,
    _target: &vfs::inode::Inode,
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
    let mut locked_dir = ramfs_inode.lock();
    if locked_dir.entries.iter().any(|entry| entry.name == new) {
        return Err(vfs::inode::RenameError::AlreadyExists);
    }

    // Find and rename the entry from the directory. If the entry is not
    // found, it return an error.
    locked_dir
        .entries
        .iter_mut()
        .find(|entry| entry.name == old)
        .map(|entry| entry.name = String::from(new))
        .ok_or(vfs::inode::RenameError::NoSuchEntry)?;
    Ok(())
}

/// Read the directory entry at the given offset.
///
/// # Errors
/// If there is no more entries in the directory, `ReaddirError::EndOfDirectory`
/// is returned.
fn readdir(
    file: &vfs::file::OpenFile,
    offset: vfs::file::Offset,
) -> Result<vfs::dirent::DirectoryEntry, vfs::file::ReaddirError> {
    let file_data = file
        .inode
        .data
        .downcast_ref::<Spinlock<InodeDirectory>>()
        .expect("Inode is not a ramfs inode");

    let locked_dir = file_data.lock();
    if offset.0 >= locked_dir.entries.len() {
        return Err(vfs::file::ReaddirError::EndOfDirectory);
    }
    Ok(locked_dir.entries[offset.0].clone())
}

#[allow(clippy::unnecessary_wraps)]
fn write(
    file: &vfs::file::OpenFile,
    buf: &[u8],
    offset: vfs::file::Offset,
) -> Result<usize, vfs::file::WriteError> {
    let file_data = file
        .inode
        .data
        .downcast_ref::<Spinlock<InodeFile>>()
        .expect("Inode is not a ramfs inode");

    // Write the buffer to the file, and extend the file if necessary.
    let mut locked_file = file_data.lock();
    let content = locked_file.content_mut();
    let offset = offset.0;

    if offset + buf.len() > content.len() {
        content.resize(offset + buf.len(), 0);
        file.inode.state.lock().size = content.len();
    }

    // Write the buffer to the file and return the written size
    content[offset..offset + buf.len()].copy_from_slice(buf);
    Ok(buf.len())
}

#[allow(clippy::unnecessary_wraps)]
fn read(
    file: &vfs::file::OpenFile,
    buf: &mut [u8],
    offset: vfs::file::Offset,
) -> Result<usize, vfs::file::ReadError> {
    let file_data = file
        .inode
        .data
        .downcast_ref::<Spinlock<InodeFile>>()
        .expect("Inode is not a ramfs inode");

    let locked_file = file_data.lock();
    let content = locked_file.content();

    // Read the buffer from the file. If the read goes beyond the end of the
    // file, the buffer is only partially written and the size readed is
    // returned.
    let len = core::cmp::min(buf.len(), content.len() - offset.0);
    buf[..len].copy_from_slice(&content[offset.0..offset.0 + len]);
    Ok(len)
}

/// Seek into the file and return the new offset.
/// TODO: This function will probably not vary much between filesystems. Maybe
/// we can make it a default implementation in the VFS?
///
/// # Errors
/// If an overflow occurs, `SeekError::Overflow` is returned.
fn seek(
    file: &vfs::file::OpenFile,
    offset: isize,
    whence: vfs::file::Whence,
) -> Result<vfs::file::Offset, vfs::file::SeekError> {
    match whence {
        vfs::file::Whence::Start => {
            let offset = offset
                .try_into()
                .map_err(|_| vfs::file::SeekError::Overflow)?;
            Ok(vfs::file::Offset(offset))
        }
        vfs::file::Whence::Current => {
            let offset = file
                .state
                .lock()
                .offset
                .0
                .checked_add_signed(offset)
                .ok_or(vfs::file::SeekError::Overflow)?;
            Ok(vfs::file::Offset(offset))
        }
        vfs::file::Whence::End => {
            let file_data = file
                .inode
                .data
                .downcast_ref::<Spinlock<InodeFile>>()
                .expect("Inode is not a ramfs inode");
            let offset = file_data
                .lock()
                .content()
                .len()
                .checked_add_signed(offset)
                .ok_or(vfs::file::SeekError::Overflow)?;
            Ok(vfs::file::Offset(offset))
        }
    }
}
