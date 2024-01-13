use self::{
    inode::Inode,
    mount::ReadInodeError,
    path::{InvalidPath, Path},
};
use crate::{
    device::Device,
    module,
    vfs::{file::OpenFileCreateInfo, inode::ROOT},
};

pub mod dirent;
pub mod file;
pub mod fs;
pub mod inode;
pub mod mount;
pub mod name;
pub mod path;
pub mod fd;

/// Setup the virtual filesystem
#[init]
pub fn setup() {
    fs::mount_root("ramfs", Device::None);
    fill_ramdisk();
}

/// Fill the ramdisk with some files and directories
fn fill_ramdisk() {
    let shell_data = module::read("/boot/shell.elf").expect("Shell executable not found");

    let root = ROOT.get().unwrap();
    root.as_directory()
        .unwrap()
        .create(root, "shell.elf")
        .expect("Failed to create shell.elf");

    let shell = lookup("/shell.elf", root, root).expect("Shell.elf created but not found");
    let file = file::OpenFile::new(OpenFileCreateInfo {
        operation: shell.file_ops.clone(),
        inode: shell,
        open_flags: file::OpenFlags::READ | file::OpenFlags::WRITE,
        data: Box::new(()),
    });

    // Write the shell to the file
    let len = file
        .as_file()
        .unwrap()
        .write(&file, shell_data, file::Offset(0))
        .expect("Failed to write to shell.elf");
    assert!(
        len == shell_data.len(),
        "Wrote {len} bytes instead of {}",
        shell_data.len()
    );
}

/// Lookup the path and return the inode associated with it.
///
/// # Errors
/// This function can fails in many ways, and each of them is described by the
/// [`LookupError`] enum.
/// 
/// # Panics
/// This function panics if an inode of one component of the path does not
/// have a superblock associated with it. This should never happen, and is
/// a serious bug if it does.
pub fn lookup(path: &str, root: &Arc<Inode>, cwd: &Arc<Inode>) -> Result<Arc<Inode>, LookupError> {
    let path = Path::new(path)?;
    let mut parent = if path.is_absolute() {
        Arc::clone(root)
    } else {
        Arc::clone(cwd)
    };

    for (i, name) in path.components.iter().enumerate() {
        let superblock = parent.superblock.upgrade().unwrap();

        let id = (parent
            .as_directory()
            .ok_or(LookupError::NotADirectory)?
            .lookup)(&parent, name.as_str())
        .map_err(|_| {
            let remaning = &path.components[i..path.components.len()];
            LookupError::NotFound(parent, Path::from(remaning))
        })?;

        let inode = superblock.get_inode(id)?;
        parent = Arc::clone(&inode);
    }

    // We return the final inode and not its parent despite the variable name
    // because the parent is set to the inode found at the end of each iteration
    // of the loop.
    Ok(parent)
}

#[derive(Debug)]
pub enum LookupError {
    /// The path could not be resolved entirely. This variant contains the
    /// last inode found before the path could not be resolved anymore, and
    /// the remaining path that could not be resolved.
    NotFound(Arc<Inode>, Path),

    /// The path is invalid. This variant contains an error describing why the
    /// path is invalid.
    InvalidPath(InvalidPath),

    /// An component of the path used as a directory is not a directory.
    NotADirectory,

    /// The filesystem is corrupted.
    CorruptedFilesystem,

    /// An I/O error occurred while resolving the path.
    IoError,
}

impl From<InvalidPath> for LookupError {
    fn from(e: InvalidPath) -> Self {
        LookupError::InvalidPath(e)
    }
}

impl From<ReadInodeError> for LookupError {
    fn from(e: ReadInodeError) -> Self {
        match e {
            // If the inode cannot be read because the filesystem says it does not
            // exist, it means the filesystem is corrupted because the inode
            // identifier was found searching the parent directory.
            ReadInodeError::DoesNotExist => LookupError::CorruptedFilesystem,
        }
    }
}
