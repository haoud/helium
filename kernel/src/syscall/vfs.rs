use crate::{
    user::{
        self,
        scheduler::{Scheduler, SCHEDULER},
        string::SyscallString,
    },
    vfs,
};

/// Open a file, specified by `path` with the given `flags`.
///
/// # Errors
/// This function can fail in many ways, and each of them is described by the
/// [`OpenError`] enum.
///
/// # Panics
/// This function panics an inode does not have a corresponding superblock. This
/// should never happen, and is a serious bug in the kernel if it does.
pub fn open(path: usize, flags: usize) -> Result<usize, OpenError> {
    let current_task = SCHEDULER.current_task();
    let root = current_task.root();
    let cwd = current_task.cwd();

    let flags = vfs::file::OpenFlags::from_bits(flags).ok_or(OpenError::InvalidFlag)?;
    let ptr = user::Pointer::<SyscallString>::from_usize(path).ok_or(OpenError::BadAddress)?;
    let path = user::String::from_raw_ptr(&ptr)
        .ok_or(OpenError::BadAddress)?
        .fetch()
        .map_err(|_| OpenError::BadAddress)?;

    let inode = match vfs::lookup(&path, &root.lock(), &cwd.lock()) {
        Ok(inode) => {
            // If the file exists and the `MUST_CREATE` flag is set, we return an error,
            // because the user has specified that the file must be created during the
            // open call.
            if flags.contains(vfs::file::OpenFlags::MUST_CREATE) {
                return Err(OpenError::AlreadyExists);
            }
            inode
        }
        Err(e) => {
            // If the user has not specified the `CREATE` or `MUST_CREATE` flag, we return
            // an error if the file does not exist.
            if !flags.contains(vfs::file::OpenFlags::CREATE)
                && !flags.contains(vfs::file::OpenFlags::MUST_CREATE)
            {
                return Err(OpenError::NoSuchFile);
            }

            match e {
                // The path could not be resolved entirely. This variant contains the
                // last inode that could be resolved and the path that could not be
                // resolved.
                // If only the last component of the path could not be resolved and
                // the `CREATE` flag is set, the kernel will attempt to create a file
                // with the given name in the parent directory
                vfs::LookupError::NotFound(parent, path) => {
                    let name = path.as_name().ok_or(OpenError::NoSuchFile)?;
                    let superblock = parent.superblock.upgrade().unwrap();
                    let id = parent
                        .as_directory()
                        .ok_or(OpenError::NotADirectory)?
                        .create(&parent, name.as_str())?;
                    superblock.get_inode(id)?
                }
                _ => return Err(OpenError::from(e)),
            }
        }
    };

    let file = vfs::file::OpenFile::new(vfs::file::OpenFileCreateInfo {
        operation: inode.file_ops.clone(),
        open_flags: flags,
        data: Box::new(()),
        inode,
    });

    let id = current_task
        .files()
        .lock()
        .insert(Arc::new(file))
        .ok_or(OpenError::TooManyFilesOpen)?;

    Ok(id.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum OpenError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// An invalid address was passed as an argument
    BadAddress,

    /// The path is invalid
    InvalidPath,

    /// An invalid flag or flags combination was passed to the syscall
    InvalidFlag,

    /// The file does not exist
    NoSuchFile,

    // One of the components of the path is not a directory
    NotADirectory,

    /// The path does not point to a file
    NotAFile,

    /// An I/O error occurred
    IoError,

    /// The file already exists
    AlreadyExists,

    /// The kernel ran out of memory while spawning the task
    OutOfMemory,

    /// The process has too many files open and cannot open any more
    TooManyFilesOpen,

    /// An unknown error occurred
    UnknownError,
}

impl From<vfs::LookupError> for OpenError {
    fn from(error: vfs::LookupError) -> Self {
        match error {
            vfs::LookupError::InvalidPath(_) | vfs::LookupError::NotADirectory => {
                OpenError::InvalidPath
            }
            vfs::LookupError::CorruptedFilesystem => OpenError::UnknownError,
            vfs::LookupError::NotFound(_, _) => OpenError::NoSuchFile,
            vfs::LookupError::IoError => OpenError::IoError,
        }
    }
}

impl From<vfs::inode::CreateError> for OpenError {
    fn from(error: vfs::inode::CreateError) -> Self {
        match error {
            vfs::inode::CreateError::AlreadyExists => Self::AlreadyExists,
        }
    }
}

impl From<vfs::mount::ReadInodeError> for OpenError {
    fn from(error: vfs::mount::ReadInodeError) -> Self {
        match error {
            vfs::mount::ReadInodeError::DoesNotExist => Self::NoSuchFile,
        }
    }
}

impl From<OpenError> for isize {
    fn from(error: OpenError) -> Self {
        -(error as isize)
    }
}

/// Close a file descriptor.
///
/// # Errors
/// This function return an error if the file descriptor is invalid.
pub fn close(fd: usize) -> Result<usize, CloseError> {
    let current_task = SCHEDULER.current_task();
    current_task
        .files()
        .lock()
        .remove(vfs::fd::Descriptor(fd))
        .ok_or(CloseError::InvalidFileDescriptor)?;

    Ok(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum CloseError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// An invalid file descriptor was passed as an argument
    InvalidFileDescriptor,

    /// An unknown error occurred
    UnknownError,
}

impl From<CloseError> for isize {
    fn from(error: CloseError) -> Self {
        -(error as isize)
    }
}
