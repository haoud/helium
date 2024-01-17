use self::{
    dentry::Dentry,
    mount::ReadInodeError,
    path::{InvalidPath, Path},
};
use crate::{device::Device, module, vfs::dentry::ROOT};
use alloc::vec;

pub mod dentry;
pub mod dirent;
pub mod fd;
pub mod file;
pub mod fs;
pub mod inode;
pub mod mount;
pub mod name;
pub mod path;
pub mod pipe;

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
    let inode = root.inode().clone();

    inode
        .as_directory()
        .unwrap()
        .create(&inode, "shell.elf")
        .expect("Failed to create shell.elf");

    let shell = lookup("/shell.elf", root, root).expect("Shell.elf created but not found");
    let file = shell
        .open(file::OpenFlags::WRITE)
        .expect("Failed to open shell.elf");

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
pub fn lookup(
    path: &str,
    root: &Arc<Dentry>,
    cwd: &Arc<Dentry>,
) -> Result<Arc<Dentry>, LookupError> {
    let path = Path::new(path)?;
    let mut parent = if path.is_absolute() {
        Arc::clone(root)
    } else {
        Arc::clone(cwd)
    };

    // We iterate over each component of the path and fetch the inode associated
    // with it. If the inode is not found, we return the last inode found and
    // the remaining path that could not be resolved.
    // We also handle the special cases of "." and "..": "." is ignored because
    // it is the current directory, and ".." is resolved to the parent of the
    // current directory during the iteration.
    for (i, name) in path.components.iter().enumerate() {
        let dentry = match name.as_str() {
            "." => parent,
            ".." => parent.parent().expect("Dentry without alive parent found"),
            _ => Dentry::fetch(&parent, name).map_err(|e| match e {
                dentry::FetchError::NotFound => {
                    let remaining = Path::from(path.components[i..].to_vec());
                    LookupError::NotFound(Arc::clone(&parent), remaining)
                }
                dentry::FetchError::NotADirectory => LookupError::NotADirectory,
                dentry::FetchError::IoError => LookupError::IoError,
            })?,
        };

        parent = dentry;
    }

    // We return the final inode and not its parent despite the variable name
    // because the parent is set to the inode found at the end of each iteration
    // of the loop.
    Ok(parent)
}

/// Read all the data of the file at the given path.
///
/// # Errors
/// This function can fails in many ways, and each of them is described by the
/// [`ReadAllError`] enum.
///
/// # Panics
/// This function panics if the opened file does not have an inode associated
/// with it. This should never happen, and is a serious bug if it does.
pub fn read_all(
    path: &str,
    root: &Arc<Dentry>,
    cwd: &Arc<Dentry>,
) -> Result<Box<[u8]>, ReadAllError> {
    let dentry = lookup(path, root, cwd).map_err(ReadAllError::LookupError)?;
    let file = dentry
        .open(file::OpenFlags::READ)
        .map_err(|_| ReadAllError::OpenError)?;

    let len = file
        .inode
        .as_ref()
        .expect("Regular open file without inode")
        .state
        .lock()
        .size;

    let mut data = vec![0; len].into_boxed_slice();
    let readed = file
        .as_file()
        .ok_or(ReadAllError::NotAFile)?
        .read(&file, &mut data, file::Offset(0))
        .map_err(|_| ReadAllError::IoError)?;

    if readed != len {
        return Err(ReadAllError::PartialRead);
    }
    Ok(data)
}

#[derive(Debug)]
pub enum ReadAllError {
    /// The path could not be resolved. This variant contains the error that
    /// occurred while resolving the path.
    LookupError(LookupError),

    /// An error occurred while opening the file.
    OpenError,

    /// The path does not point to a file.
    NotAFile,

    /// The file could not be read entirely.
    PartialRead,

    /// An I/O error occurred while reading the file.
    IoError,
}

#[derive(Debug)]
pub enum LookupError {
    /// The path could not be resolved entirely. This variant contains the
    /// last inode found before the path could not be resolved anymore, and
    /// the remaining path that could not be resolved.
    NotFound(Arc<Dentry>, Path),

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
