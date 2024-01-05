use super::inode;

/// A directory entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DirectoryEntry {
    /// The name of the entry.
    pub name: String,

    /// The kind of this entry.
    pub kind: Kind,

    /// The inode associated with this entry.
    pub inode: inode::Identifier,
}

/// The kind of a directory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    BlockDevice,
    CharDevice,
    Directory,
    File,
}

impl From<inode::Kind> for Kind {
    fn from(kind: inode::Kind) -> Self {
        match kind {
            inode::Kind::BlockDevice(_) => Self::BlockDevice,
            inode::Kind::CharDevice(_) => Self::CharDevice,
            inode::Kind::Directory => Self::Directory,
            inode::Kind::File => Self::File,
        }
    }
}
