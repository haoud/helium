use crate::vfs::{self, dirent::DirectoryEntry};
use alloc::vec;
use core::sync::atomic::{AtomicU64, Ordering};
use hashbrown::HashMap;

pub mod interface;

pub struct Superblock {
    inodes: HashMap<vfs::inode::Identifier, Arc<vfs::inode::Inode>>,
}

impl Superblock {
    /// Get the root inode of the file system.
    ///
    /// # Panics
    /// Panics if the root inode is not found. This should never happen.
    #[must_use]
    pub fn get_root_inode(&self) -> Arc<vfs::inode::Inode> {
        self.inodes
            .get(&self.get_root_inode_id())
            .expect("Root inode not found")
            .clone()
    }

    /// Get the root inode identifier of the file system.
    #[must_use]
    pub const fn get_root_inode_id(&self) -> vfs::inode::Identifier {
        vfs::inode::Identifier(0)
    }
}

impl Default for Superblock {
    fn default() -> Self {
        Self {
            inodes: HashMap::new(),
        }
    }
}

/// The data that a directory inode contains. It is just a vector of directory
/// entries.
pub struct InodeDirectory {
    entries: Vec<vfs::dirent::DirectoryEntry>,
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
    pub fn new(this: &vfs::inode::Inode, parent: vfs::inode::Identifier) -> Self {
        assert!(this.kind == vfs::inode::Kind::Directory);
        assert!(this.state.lock().links == 0);

        this.state.lock().links = 1;
        Self {
            entries: vec![
                vfs::dirent::DirectoryEntry {
                    inode: this.id,
                    kind: vfs::dirent::Kind::Directory,
                    name: String::from("."),
                    offset: 1,
                },
                DirectoryEntry {
                    inode: parent,
                    kind: vfs::dirent::Kind::Directory,
                    name: String::from(".."),
                    offset: 1,
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
    /// # Panics
    /// Panics if the entry already exists.
    pub fn add_entry(&mut self, inode: &vfs::inode::Inode, name: String) {
        assert!(self.get_entry(&name).is_none());

        inode.state.lock().links += 1;
        self.entries.push(vfs::dirent::DirectoryEntry {
            kind: vfs::dirent::Kind::from(inode.kind),
            inode: inode.id,
            offset: 1,
            name,
        });
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

/// Register the RAM filesystem into the VFS.
pub fn register() {
    vfs::fs::register(vfs::fs::Filesystem::new(
        "ramfs",
        &interface::FS_OPS,
        Box::new(()),
    ));
}

/// Generate a new unique inode identifier.
pub fn generate_inode_id() -> vfs::inode::Identifier {
    static INODE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);
    vfs::inode::Identifier(INODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
}
