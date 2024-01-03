/// FIXME: Inode link count
use alloc::{string::String, vec, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};
use hashbrown::HashMap;

/// A global counter that is used to generate unique inode identifiers.
static INODE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

///
pub struct Superblock {
    inodes: HashMap<InodeIdentifier, Inode>,
}

impl Superblock {
    /// Create a new superblock with a root inode. Basically, this create a new filesystem
    /// inside the RAM of the kernel.
    #[must_use]
    pub fn new() -> Self {
        let root = Inode::create(InodeIdentifier(0), InodeKind::Directory);

        let mut inodes = HashMap::new();
        inodes.insert(root.id, root);

        Self { inodes }
    }

    /// Get a mutable reference to the root inode of the file system.
    ///
    /// # Panics
    /// Panics if the root inode is not found. This should never happen.
    #[must_use]
    pub fn get_root_inode_mut(&mut self) -> &mut Inode {
        self.inodes
            .get_mut(&InodeIdentifier(0))
            .expect("Root inode not found")
    }

    /// Get the root inode of the file system.
    ///
    /// # Panics
    /// Panics if the root inode is not found. This should never happen.
    #[must_use]
    pub fn get_root_inode(&self) -> &Inode {
        self.inodes
            .get(&InodeIdentifier(0))
            .expect("Root inode not found")
    }
}

impl Default for Superblock {
    fn default() -> Self {
        Self::new()
    }
}

/// An unique identifier for an inode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InodeIdentifier(pub u64);

/// An inode is a data structure that represents a file system object. It contains
/// the metadata of the object and a reference to the data that the object contains.
/// The data is not stored in the inode itself, but in a separate data structure.
/// This allows multiple inodes to refer to the same data, which is useful for
/// hard links.
pub struct Inode {
    parent: InodeIdentifier,
    id: InodeIdentifier,
    link_count: u64,
    kind: InodeKind,
    data: InodeData,
}

impl Inode {
    /// Create a new inode with the given parent inode and kind. If the inode kind
    /// is a directory, the inode will be initialized with default entries (`.`
    /// and `..`). If the inode kind is a file, the inode will be initialized with
    /// no content.
    #[must_use]
    pub fn create(parent: InodeIdentifier, kind: InodeKind) -> Self {
        let id = Self::generate_identifier();
        let data = InodeData::File(InodeFile::empty());

        let mut inode = Self {
            id,
            parent,
            link_count: 0,
            kind,
            data,
        };

        inode.data = match kind {
            InodeKind::Directory => InodeData::Directory(InodeDirectory::new(&mut inode, parent)),
            InodeKind::File => InodeData::File(InodeFile::empty()),
        };

        inode
    }

    /// Generate a new unique inode identifier.
    #[must_use]
    pub fn generate_identifier() -> InodeIdentifier {
        InodeIdentifier(INODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    #[must_use]
    pub fn as_mut_directory(&mut self) -> Option<&mut InodeDirectory> {
        match &mut self.data {
            InodeData::Directory(directory) => Some(directory),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_mut_file(&mut self) -> Option<&mut InodeFile> {
        match &mut self.data {
            InodeData::File(file) => Some(file),
            _ => None,
        }
    }

    /// Assuming that the inode is a directory, get a reference to the directory
    /// data.
    #[must_use]
    pub fn as_directory(&self) -> Option<&InodeDirectory> {
        match &self.data {
            InodeData::Directory(directory) => Some(directory),
            _ => None,
        }
    }

    /// Assuming that the inode is a file, get a reference to the file data.
    #[must_use]
    pub fn as_file(&self) -> Option<&InodeFile> {
        match &self.data {
            InodeData::File(file) => Some(file),
            _ => None,
        }
    }
}

/// The kind of an inode. It refers to the type of the data that the inode
/// contains. For now, we only support directories and files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InodeKind {
    Directory,
    File,
}

/// The data that an inode contains. It depends on the kind of the inode, allowing
/// different types of data to be stored in the same structure.
#[non_exhaustive]
pub enum InodeData {
    Directory(InodeDirectory),
    File(InodeFile),
}

/// The data that a directory inode contains. It is just a vector of directory
/// entries.
pub struct InodeDirectory {
    entries: Vec<DirectoryEntry>,
}

impl InodeDirectory {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Create a new directory inode with default entries (`.`, which refers to
    /// the inode itself, and `..`, which refers to the parent inode).
    ///
    /// # Panics
    /// Panics if the inode is not a directory or if the inode already has entries.
    #[must_use]
    pub fn new(this: &mut Inode, parent: InodeIdentifier) -> Self {
        assert!(this.kind == InodeKind::Directory);
        assert!(this.link_count == 0);

        this.link_count = 2;
        Self {
            entries: vec![
                DirectoryEntry {
                    inode: this.id,
                    kind: InodeKind::Directory,
                    name: String::from("."),
                },
                DirectoryEntry {
                    inode: parent,
                    kind: InodeKind::Directory,
                    name: String::from(".."),
                },
            ],
        }
    }

    /// Find the entry with the given name in the directory. If the entry is not
    /// found, return `None`.
    #[must_use]
    pub fn get_entry(&self, name: &str) -> Option<&DirectoryEntry> {
        self.entries.iter().find(|entry| entry.name == name)
    }

    /// Add an entry to the directory. If an entry with the same name already
    /// exists, return an error.
    ///
    /// # Errors
    /// Return an error if an entry with the same name already exists.
    pub fn add_entry(&mut self, inode: &Inode, name: String) -> Result<(), ()> {
        if self.get_entry(&name).is_some() {
            return Err(());
        }

        self.entries.push(DirectoryEntry {
            inode: inode.id,
            kind: inode.kind,
            name,
        });
        Ok(())
    }
}

/// The data that a file inode contains. It is just a vector of bytes.
pub struct InodeFile {
    content: Vec<u8>,
}

impl InodeFile {
    /// Create a new file inode with no content.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            content: Vec::new(),
        }
    }

    #[must_use]
    pub fn new(content: Vec<u8>) -> Self {
        Self { content }
    }

    /// Get a mutable reference to the content of the file.
    #[must_use]
    pub fn content_mut(&mut self) -> &mut Vec<u8> {
        &mut self.content
    }

    /// Get a reference to the content of the file.
    #[must_use]
    pub fn content(&self) -> &[u8] {
        &self.content
    }
}

impl Default for InodeFile {
    fn default() -> Self {
        Self::empty()
    }
}

/// A directory entry. It contains the name of the entry and the inode identifier
/// of the inode that the entry refers to.
pub struct DirectoryEntry {
    inode: InodeIdentifier,
    kind: InodeKind,
    name: String,
}
