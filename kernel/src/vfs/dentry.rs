use super::{
    file::{self, OpenFile, OpenFileCreateInfo, OpenFlags},
    inode::{self, Inode},
    mount,
    name::Name,
};
use alloc::sync::Weak;

/// The root dentry of the filesystem tree.
pub static ROOT: Once<Arc<Spinlock<Dentry>>> = Once::new();

/// A dentry is a directory entry. It is a node in the filesystem tree, and
/// contains the name of the file, the inode associated with the file, and
/// pointers to its parent and children.
/// 
/// A dentry object can only be referenced once in the dentry cache. However,
/// the underlying inode can be referenced multiple times, for example if the
/// file has multiple hard links.
#[derive(Debug)]
pub struct Dentry {
    /// The name of this dentry.
    name: Name,

    /// The inode associated with this dentry. An same inode may be associated
    /// with multiple dentries (hard links).
    inode: Arc<Inode>,

    /// The parent dentry of this dentry. Every dentry has a parent, even the
    /// root dentry, which has itself as its parent.
    parent: Weak<Spinlock<Dentry>>,

    /// The children of this dentry. This is a list of all dentries that have
    /// this dentry as their parent.
    children: Vec<Arc<Spinlock<Dentry>>>,
}

impl Dentry {
    /// Create a new dentry with the given name and inode. This dentry does not
    /// have a parent, and must be connected to a parent before being used.
    #[must_use]
    pub fn new(name: Name, inode: Arc<Inode>) -> Self {
        Self {
            children: Vec::new(),
            parent: Weak::default(),
            inode,
            name,
        }
    }

    /// Create a new root dentry. This is similar to [`Self::new`], but sets the
    /// parent of the dentry to itself instead of leaving it unset.
    #[must_use]
    pub fn root(name: Name, inode: Arc<Inode>) -> Arc<Spinlock<Self>> {
        let dentry = Arc::new(Spinlock::new(Self::new(name, inode)));
        dentry.lock().parent = Arc::downgrade(&dentry);
        dentry
    }

    /// Open the inode associated with this dentry.
    ///
    /// Please note that this function does not perform any checks: this is the caller
    /// responsibility to ensure that.
    ///
    /// # Errors
    /// Currently, this function does not return any error. However, this may change
    /// in the future.
    pub fn open(&self, flags: OpenFlags) -> Result<OpenFile, OpenError> {
        Ok(file::OpenFile::new(OpenFileCreateInfo {
            operation: self.inode.file_ops.clone(),
            inode: self.inode.clone(),
            open_flags: flags,
            data: Box::new(()),
        }))
    }

    /// Get the parent of this dentry.
    #[must_use]
    pub fn parent(&self) -> Option<Arc<Spinlock<Dentry>>> {
        self.parent.upgrade()
    }

    /// Get the inode associated with this dentry.
    #[must_use]
    pub fn inode(&self) -> &Arc<Inode> {
        &self.inode
    }

    /// Get the name of this dentry.
    #[must_use]
    pub fn name(&self) -> &Name {
        &self.name
    }

    /// Fetch a child of this dentry by name. It will first try to find the child
    /// in the dentry cache, and if it is not found, it will look it up in the
    /// filesystem tree. If the child is found in the cache, the inode will be loaded
    /// into the cache to speed up future lookups.
    ///
    /// # Errors
    /// - `FetchError::NotADirectory`: The inode associated with this dentr
    /// is not a directory, and therefore cannot have children.
    /// - `FetchError::NotFound`: The child could not be found, either in the
    /// dentry cache or in the filesystem tree.
    /// - `FetchError::IoError`: The child could not be fetched because of an
    /// I/O error.
    ///
    /// # Panics
    /// Panics if the inode cannot be read because the filesystem is corrupted, if the
    /// inode does not have a superblock or if the dentry could not be connected to its
    /// parent. The last two cases should never happen and are serious kernel bugs.
    pub fn fetch(
        dentry: &Arc<Spinlock<Self>>,
        name: &Name,
    ) -> Result<Arc<Spinlock<Self>>, FetchError> {
        let locked_dentry = dentry.lock();
        match locked_dentry.lookup(name) {
            Ok(dentry) => return Ok(dentry),
            Err(e) => match e {
                LookupError::NotADirectory => return Err(FetchError::NotADirectory),
                LookupError::NotFound => {}
            },
        }

        let superblock = locked_dentry
            .inode
            .superblock
            .upgrade()
            .expect("Inode without superblock");

        let id = locked_dentry
            .inode
            .as_directory()
            .ok_or(FetchError::NotADirectory)?
            .lookup(&locked_dentry.inode, name.as_str())
            .map_err(|e| match e {
                inode::LookupError::NoSuchEntry => FetchError::NotFound,
            })?;

        core::mem::drop(locked_dentry);

        // Create the dentry, connect it to its parent and return the dentry.
        // We assume that the `create_and_connect_child` function will never
        // fail because we just checked that the child does not exist and
        // we have held the lock on the parent dentry since then.
        let inode = superblock.get_inode(id)?;
        let name = name.clone();
        Ok(Self::create_and_connect_child(dentry, inode, name).unwrap())
    }

    /// Create a new file in this dentry with the given name, load it into the
    /// dentry cache and return it.
    ///
    /// # Errors
    ///  - `CreateFetchError::NotADirectory`: The inode associated with this dentry
    ///  is not a directory, and therefore cannot have children.
    /// - `CreateFetchError::AlreadyExists`: A child with the same name already exists.
    ///
    /// # Panics
    /// Panics if the inode associated with this dentry does not have a superblock, or
    /// if this function fails to connect the created dentry to this dentry. This should
    /// never happen and is a serious kernel bug.
    pub fn create_and_fetch_file(
        dentry: Arc<Spinlock<Self>>,
        name: Name,
    ) -> Result<Arc<Spinlock<Self>>, CreateFetchError> {
        // Search for the file in the dentry cache and in the underlying filesystem.
        // If a file with the same name already exists, return an error.
        let locked_dentry = dentry.lock();
        match Self::fetch(&dentry, &name) {
            Err(FetchError::NotFound) => {}
            Err(FetchError::NotADirectory) => return Err(CreateFetchError::NotADirectory),
            Err(FetchError::IoError) => return Err(CreateFetchError::IoError),
            Ok(_) => return Err(CreateFetchError::AlreadyExists),
        }

        let superblock = locked_dentry
            .inode
            .superblock
            .upgrade()
            .expect("Inode without superblock");

        let id = locked_dentry
            .inode
            .as_directory()
            .ok_or(CreateFetchError::NotADirectory)?
            .create(&locked_dentry.inode, name.as_str())?;

        core::mem::drop(locked_dentry);

        let inode = superblock.get_inode(id)?;
        let child = Arc::new(Spinlock::new(Dentry::new(name, inode)));

        Self::connect_child(&dentry, child).unwrap();
        Ok(dentry)
    }

    /// Find a child of this dentry by name.
    ///
    /// The dentry cache only contains parts of the filesystem tree. If the child is not found
    /// in the cache, it MUST be looked up in the filesystem tree to ensure that the entry really
    /// does not exist. If the child is found in the cache, the inode should be inserted into the
    /// cache to speed up future lookups.
    ///
    /// # Errors
    ///  - `LookupError::NotADirectory`: The inode associated with this dentry
    ///   is not a directory, and therefore cannot have children.
    ///  - `LookupError::NotFound`: The child could not be found.
    ///
    /// # Panics
    /// Panics if this entry does not have a parent. This should never happen
    /// because every dentry must have a parent, even the root dentry, and is
    /// a serious kernel bug.
    pub fn lookup(&self, name: &Name) -> Result<Arc<Spinlock<Dentry>>, LookupError> {
        if self.inode.kind != inode::Kind::Directory {
            return Err(LookupError::NotADirectory);
        }

        self.children
            .iter()
            .find(|child| child.lock().name == *name)
            .cloned()
            .ok_or(LookupError::NotFound)
    }

    /// Connect a child to this dentry.
    ///
    /// # Errors
    /// - `ConnectError::NotADirectory`: The inode associated with this dentry
    ///  is not a directory, and therefore cannot have children.
    /// - `ConnectError::AlreadyExists`: The parent already has a child with the
    /// same name.
    pub fn connect_child(
        dentry: &Arc<Spinlock<Dentry>>,
        child: Arc<Spinlock<Dentry>>,
    ) -> Result<(), ConnectError> {
        let mut locked_dentry = dentry.lock();
        if locked_dentry.inode.kind != inode::Kind::Directory {
            return Err(ConnectError::NotADirectory);
        }

        {
            let mut locked_child = child.lock();
            if locked_child.parent.upgrade().is_some() {
                return Err(ConnectError::AlreadyConnected);
            }

            if locked_dentry
                .children
                .iter()
                .any(|entry| entry.lock().name == locked_child.name)
            {
                return Err(ConnectError::AlreadyExists);
            }

            locked_child.parent = Arc::downgrade(dentry);
        }

        locked_dentry.children.push(child);
        Ok(())
    }

    /// Create a new dentry with the given name and inode, connect it to this dentry
    /// and return the created dentry. This is simply a shortcut for creating a new
    /// dentry and calling [`Self::connect_child`] on it.
    ///
    /// # Errors
    /// See [`Self::connect_child`] for the list of errors that can be returned
    /// by this function.
    pub fn create_and_connect_child(
        dentry: &Arc<Spinlock<Dentry>>,
        inode: Arc<Inode>,
        name: Name,
    ) -> Result<Arc<Spinlock<Dentry>>, ConnectError> {
        let child = Arc::new(Spinlock::new(Self::new(name, inode)));
        Self::connect_child(dentry, child.clone())?;
        Ok(child)
    }

    /// Disconnect the child with the given name from this dentry.
    ///
    /// Identically to Linux, this function can remove a dentry from its
    /// parent that is still in use. This is because the dentry will be
    /// removed from the dentry cache but will only be freed when all
    /// references to it are dropped.
    ///
    /// # Errors
    /// - `DisconnectError::Busy`: The dentry is still used and cannot be
    /// disconnected. This happens when the dentry still have children
    /// attached to it, i.e when a user try to remove a directory that is
    /// not empty.
    /// - `DisconnectError::NotFound`: There is no dentry with the given
    /// name in the children list.
    ///
    /// # Panics
    /// Panics if one of the following conditions happens:
    ///  - The dentry is not found in its parent children list
    ///  - The dentry does not have an alive parent
    ///
    /// All these cases should never happen and are serious kernel bugs.
    pub fn disconnect_child(
        &mut self,
        name: &Name,
    ) -> Result<Arc<Spinlock<Self>>, DisconnectError> {
        let index = self
            .children
            .iter()
            .position(|child| child.lock().name == *name)
            .ok_or(DisconnectError::NotFound)?;

        {
            let mut child = self.children[index].lock();
            if !child.children.is_empty() {
                return Err(DisconnectError::Busy);
            }
            child.parent = Weak::default();
        }

        Ok(self.children.swap_remove(index))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OpenError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LookupError {
    /// The inode associated with this dentry is not a directory, and therefore
    /// cannot have children.
    NotADirectory,

    /// The child could not be found.
    NotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FetchError {
    /// The inode associated with this dentry is not a directory, and therefore
    /// cannot have children.
    NotADirectory,

    /// The child could not be found.
    NotFound,

    /// The child could not be fetched because of an I/O error.
    IoError,
}

impl From<mount::ReadInodeError> for FetchError {
    fn from(error: mount::ReadInodeError) -> Self {
        match error {
            mount::ReadInodeError::DoesNotExist => panic!("Filesystem corrupted"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreateFetchError {
    /// The inode associated with this dentry is not a directory, and therefore
    /// cannot have children.
    NotADirectory,

    /// A child with the same name already exists.
    AlreadyExists,

    /// The child could not be fetched because of an I/O error.
    IoError,
}

impl From<inode::CreateError> for CreateFetchError {
    fn from(error: inode::CreateError) -> Self {
        match error {
            inode::CreateError::AlreadyExists => CreateFetchError::AlreadyExists,
        }
    }
}

impl From<mount::ReadInodeError> for CreateFetchError {
    fn from(error: mount::ReadInodeError) -> Self {
        match error {
            mount::ReadInodeError::DoesNotExist => panic!("Filesystem corrupted"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConnectError {
    /// The dentry is already connected to a parent.
    AlreadyConnected,

    /// The inode associated with this dentry is not a directory, and therefore
    /// cannot have children.
    NotADirectory,

    /// The parent already has a child with the same name.
    AlreadyExists,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DisconnectError {
    /// There is no dentry with the given name in the children list.
    NotFound,

    /// The dentry is still used and cannot be disconnected. This happens when
    /// the dentry dentry still contains children, i.e when a user try to remove
    /// a directory that is not empty.
    Busy,
}

/// Setup the root dentry.
#[init]
pub fn setup(root: Arc<Inode>) {
    ROOT.call_once(|| {
        let name = Name::new("ROOT".to_string()).unwrap();
        Dentry::root(name, root)
    });
}
