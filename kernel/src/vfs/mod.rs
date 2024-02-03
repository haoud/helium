use self::{dentry::Dentry, mount::ReadInodeError};
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

pub use self::name::*;
pub use self::path::*;

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

    // Create the /bin directory
    inode
        .as_directory()
        .unwrap()
        .mkdir(&inode, "bin")
        .expect("Failed to create /bin");

    // Create the /usr directory
    inode
        .as_directory()
        .unwrap()
        .mkdir(&inode, "usr")
        .expect("Failed to create /usr");

    inode
        .as_directory()
        .unwrap()
        .create(&inode, "shell.elf")
        .expect("Failed to create shell.elf");

    let shell = lookup(
        &Path::new("/shell.elf").unwrap(),
        root,
        root,
        LookupFlags::empty(),
    )
    .expect("Shell.elf created but not found");

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

bitflags::bitflags! {
    /// Flags to control the behavior of the lookup operation.
    #[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct LookupFlags: u32 {
        /// Return the parent of the last component of the path. If the path has
        /// only one component, return the parent of the current directory. If the
        /// parent cannot be resolved, return an error.
        const PARENT = 1 << 0;

        /// The dentry resolved must be a directory. This is useful used in
        /// combination with the `PARENT` flag to ensure that the parent of the
        /// last component of the path is a directory.
        const DIRECTORY = 1 << 1;
    }
}

/// Resolve the given path to a dentry.
///
/// # Errors
/// This function can fail in many ways, and each of them is described by the
/// [`LookupError`] enum.
///
/// # Panics
/// This function panics if a dentry without an alive parent is found. This
/// should never happen, and is a serious bug if it does.
pub fn lookup(
    path: &Path,
    root: &Arc<Dentry>,
    cwd: &Arc<Dentry>,
    flags: LookupFlags,
) -> Result<Arc<Dentry>, LookupError> {
    // The parent of the current component of the path. It is initialized to
    // the root or the current directory, depending on whether the path is
    // absolute or relative.
    let mut parent = if path.is_absolute() {
        Arc::clone(root)
    } else {
        Arc::clone(cwd)
    };

    // The components of the path to resolve. If the `PARENT` flag is set, the
    // last component is not resolved because this is the parent component that
    // we want to return.
    let components = if flags.contains(LookupFlags::PARENT) {
        &path.components[..path.components.len() - 1]
    } else {
        &path.components
    };

    for (i, name) in components.iter().enumerate() {
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

    if flags.contains(LookupFlags::DIRECTORY) && parent.inode().kind != inode::Kind::Directory {
        return Err(LookupError::NotADirectory);
    }

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
    path: &Path,
    root: &Arc<Dentry>,
    cwd: &Arc<Dentry>,
) -> Result<Box<[u8]>, ReadAllError> {
    let dentry =
        lookup(path, root, cwd, LookupFlags::empty()).map_err(ReadAllError::LookupError)?;
    let file = dentry
        .open(file::OpenFlags::READ)
        .map_err(|_| ReadAllError::OpenError)?;

    let len = file
        .dentry
        .as_ref()
        .expect("Regular open file without dentry")
        .inode()
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

    /// An component of the path used as a directory is not a directory.
    NotADirectory,

    /// The filesystem is corrupted.
    CorruptedFilesystem,

    /// An I/O error occurred while resolving the path.
    IoError,
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
