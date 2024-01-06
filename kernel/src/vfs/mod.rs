use self::{
    inode::Inode,
    mount::ReadInodeError,
    path::{InvalidPath, Path},
};
use crate::{
    device::Device,
    vfs::{
        file::{OpenFile, OpenFileCreateInfo, OpenFlags},
        inode::ROOT,
    },
};

pub mod dirent;
pub mod file;
pub mod fs;
pub mod inode;
pub mod mount;
pub mod name;
pub mod path;

/// Setup the virtual filesystem
#[init]
pub fn setup() {
    fs::mount_root("ramfs", Device::None);
    test();
}

pub fn test() {
    let root = ROOT.get().unwrap();

    log::debug!("Testing vfs and ramfs");

    log::debug!("{:#?}", lookup("/"));
    log::debug!("{:#?}", lookup("/dev"));
    log::debug!("{:#?}", (root.as_directory().unwrap().mkdir)(root, "dev"));
    log::debug!("{:#?}", lookup("/dev"));
    log::debug!("{:#?}", (root.as_directory().unwrap().rmdir)(root, "dev"));
    log::debug!("{:#?}", lookup("/dev"));
    log::debug!(
        "{:#?}",
        (root.as_directory().unwrap().create)(root, "test.txt")
    );

    let inode = lookup("/test.txt").unwrap();
    let file = OpenFile::new(OpenFileCreateInfo {
        inode: Arc::clone(&inode),
        operation: inode.file_ops.clone(),
        open_flags: OpenFlags::READ | OpenFlags::WRITE,
        data: Box::new(()),
    });

    (file.as_file().unwrap().write)(&file, b"Hello world!", file.state.lock().offset).unwrap();

    log::debug!("END");
}

/// Lookup the path and return the inode associated with it.
///
/// # Errors
/// This function can fails in many ways, and each of them is described by the
/// [`LookupError`] enum.
///
/// # Panics
/// Panics if the path is not absolute (unimplemented yet).
pub fn lookup(path: &str) -> Result<Arc<Inode>, LookupError> {
    let path = Path::new(path)?;
    assert!(path.is_absolute(), "Relative paths are not supported yet");

    let mut parent = Arc::clone(ROOT.get().unwrap());
    for (i, name) in path.components.iter().enumerate() {
        let superblock = parent.superblock.upgrade().unwrap();

        let id = (parent
            .as_directory()
            .ok_or(LookupError::NotADirectory)?
            .lookup)(&parent, name.as_str())
        .map_err(|_| {
            let remaning = &path.components[i..path.components.len()];
            LookupError::NotFound(Path::from(remaning))
        })?;

        let inode = superblock.get_inode(id)?;
        parent = Arc::clone(&inode);
    }

    // We return the final inode and not its parent despite the variable name
    // because the parent is set to the inode found at the end of each iteration
    // of the loop.
    Ok(parent)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LookupError {
    /// The path could not be resolved entirely. This variant contains the
    /// unresolved part of the path.
    NotFound(Path),

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
