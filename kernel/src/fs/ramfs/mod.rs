//! TODO: Directly use the VFS inode type to store the RAM inode type, instead of
//! using a separate structure.
use crate::{
    device::Device,
    time::unix::UnixTime,
    vfs::{self, dirent::DirectoryEntry},
};
use alloc::{sync::Weak, vec};
use core::sync::atomic::{AtomicU64, Ordering};
use hashbrown::HashMap;

pub mod interface;

/// A global counter that is used to generate unique inode identifiers.
static INODE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

///
pub struct Superblock {
    inodes: HashMap<vfs::inode::Identifier, Arc<vfs::inode::Inode>>,
}

impl Superblock {
    /// Create a new superblock with a root inode. Basically, this create a new filesystem
    /// inside the RAM of the kernel.
    ///
    /// FIXME: Weak::default() is not a valid inode identifier.
    #[must_use]
    pub fn new() -> Self {
        let root = vfs::inode::Inode::new(
            Weak::default(),
            vfs::inode::InodeCreateInfo {
                id: vfs::inode::Identifier(generate_inode_id().0),
                device: Device::None,
                kind: vfs::inode::Kind::Directory,
                inode_ops: vfs::inode::Operation::Directory(&interface::INODE_DIR_OPS),
                file_ops: vfs::file::Operation::Directory(&interface::FILE_DIRECTORY_OPS),
                state: vfs::inode::InodeState {
                    modification_time: UnixTime::now(),
                    access_time: UnixTime::now(),
                    change_time: UnixTime::now(),
                    links: 1,
                    size: 0,
                },
                data: Box::new(Spinlock::new(InodeDirectory::empty())),
            },
        );

        let mut inodes = HashMap::new();
        inodes.insert(root.id, Arc::new(root));
        Self { inodes }
    }

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
        Self::new()
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

        this.state.lock().links = 2;
        Self {
            entries: vec![
                vfs::dirent::DirectoryEntry {
                    inode: this.id,
                    kind: vfs::dirent::Kind::Directory,
                    name: String::from("."),
                },
                DirectoryEntry {
                    inode: parent,
                    kind: vfs::dirent::Kind::Directory,
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
    pub fn add_entry(&mut self, inode: &vfs::inode::Inode, name: String) -> Result<(), ()> {
        if self.get_entry(&name).is_some() {
            return Err(());
        }

        inode.state.lock().links += 1;
        self.entries.push(vfs::dirent::DirectoryEntry {
            kind: vfs::dirent::Kind::from(inode.kind),
            inode: inode.id,
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

/// Register the RAM filesystem into the VFS.
pub fn register() {
    vfs::fs::register(vfs::fs::Filesystem::new(
        "ramfs",
        &interface::FS_OPS,
        Box::new(()),
    ));
}

pub fn generate_inode_id() -> vfs::inode::Identifier {
    vfs::inode::Identifier(INODE_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
}
