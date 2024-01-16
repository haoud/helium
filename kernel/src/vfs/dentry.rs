use super::{
    file::{self, OpenFile, OpenFileCreateInfo, OpenFlags},
    inode::{self, Inode},
    name::Name,
};
use alloc::sync::Weak;

/// The root dentry of the filesystem tree.
pub static ROOT: Once<Arc<Spinlock<Dentry>>> = Once::new();

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
    /// Create a new root dentry. This is similar to [`Self::new`], but sets the
    /// parent of the dentry to itself instead of leaving it unset.
    #[must_use]
    pub fn root(name: Name, inode: Arc<Inode>) -> Arc<Spinlock<Self>> {
        let dentry = Arc::new(Spinlock::new(Self::new(name, inode)));
        dentry.lock().parent = Arc::downgrade(&dentry);
        dentry
    }

    #[must_use]
    pub fn new(name: Name, inode: Arc<Inode>) -> Self {
        Self {
            children: Vec::new(),
            parent: Weak::default(),
            inode,
            name,
        }
    }

    /// Open the inode associated with this dentry.
    ///
    /// Please note that this function does not perform any checks: the caller
    ///
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
    pub fn connect_child(&mut self, child: Arc<Spinlock<Dentry>>) -> Result<(), ConnectError> {
        if self.inode.kind != inode::Kind::Directory {
            return Err(ConnectError::NotADirectory);
        }

        if child.lock().parent.upgrade().is_some() {
            return Err(ConnectError::AlreadyConnected);
        }

        if self
            .children
            .iter()
            .any(|entry| entry.lock().name == child.lock().name)
        {
            return Err(ConnectError::AlreadyExists);
        }

        self.children.push(child);
        Ok(())
    }

    /// Create a new dentry with the given name and inode, connect it to this dentry
    /// and return the created dentry.
    ///
    /// # Errors
    /// See [`Self::connect_child`] for the list of errors that can be returned
    /// by this function.
    pub fn create_and_connect_child(
        &mut self,
        inode: Arc<Inode>,
        name: Name,
    ) -> Result<Arc<Spinlock<Dentry>>, ConnectError> {
        let child = Arc::new(Spinlock::new(Self::new(name, inode)));
        self.connect_child(child.clone())?;
        Ok(child)
    }

    /// Disconnect the child with the given name from this dentry.
    ///
    /// # Errors
    /// - `DisconnectError::Busy`: The dentry is still used and cannot be
    /// disconnected. This happens when the dentry is still referenced by a
    /// file descriptor, or when the dentry still contains children.
    /// This happens when the dentry strong count is greater than 1, so be careful
    /// if you clone the dentry before trying to disconnect it.
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

        let child = self.children[index].lock();
        if !child.children.is_empty() {
            return Err(DisconnectError::Busy);
        }

        core::mem::drop(child);
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
