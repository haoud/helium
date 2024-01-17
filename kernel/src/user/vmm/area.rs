use crate::{
    vfs::file::File,
    x86_64::paging::table::{PageEntryFlags, PageFaultErrorCode},
};
use addr::user::UserVirtual;
use bitflags::bitflags;
use core::ops::Range;
use typed_builder::TypedBuilder;

/// A virtual memory area. This structure is used to represent a range of a
/// virtual memory range that is mapped in a task address space.
#[derive(TypedBuilder, Debug, Clone)]
pub struct Area {
    /// The range of virtual addresses that are mapped by this area. The start address
    /// of the range must be page-aligned.
    range: Range<UserVirtual>,

    /// The access rights of this area.
    access: Access,

    /// The flags of this area.
    flags: Flags,

    /// An offset in the ressource associated with this area. This is used for
    /// file areas to determine where the file content should be copied in the
    /// area.
    offset: usize,

    /// The kind of this area.
    kind: Type,
}

impl Area {
    /// Change the range of this area.
    pub fn set_range(&mut self, range: Range<UserVirtual>) {
        self.range = range;
    }

    /// Return the range of virtal memory used by this area.
    #[must_use]
    pub fn range(&self) -> &Range<UserVirtual> {
        &self.range
    }

    /// Return the access rights of this area.
    #[must_use]
    pub fn access(&self) -> Access {
        self.access
    }

    /// Return the ressource offset of this area.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Return the flags of this area.
    #[must_use]
    pub fn flags(&self) -> Flags {
        self.flags
    }

    /// Return the base address of this area.
    #[must_use]
    pub fn base(&self) -> UserVirtual {
        self.range.start
    }

    /// Return the type of this area.
    #[must_use]
    pub fn kind(&self) -> &Type {
        &self.kind
    }

    /// Return true if this area length is zero, false otherwise.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Return the length of the mapping of this area.
    #[must_use]
    pub fn len(&self) -> usize {
        usize::from(self.range.end) - usize::from(self.range.start)
    }
}

#[derive(Debug, Clone)]
pub enum Type {
    /// An anonymous area is an area that is not backed by any file and is
    /// initialized with zeros when it is mapped.
    Anonymous,

    /// A file area is an area that is backed by a file. When it is mapped,
    /// the content of the file is copied into the area and can be copied
    /// back to the file when the area is unmapped if some flags are set
    /// during the mapping.
    File(Arc<File>),
}

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Access : u64 {
        /// The area can be readed
        const READ = 1 << 0;

        /// The area can be written. On x86_64, this flag implicitly implies
        /// the `READ` flag.
        const WRITE = 1 << 1;

        /// The area can be executed. On x86_64, this flag implicitly implies
        /// the `READ` flag.
        const EXECUTE = 1 << 2;
    }

    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Flags : u64 {
        /// The area range is fixed and cannot be moved. This flags is only
        /// used when creating an area and is ignored afterwards.
        const FIXED = 1 << 0;

        /// The area is shared between multiple processes.
        const SHARED = 1 << 1;

        /// The area can grow up if needed.
        const GROW_UP = 1 << 2;

        /// The area can grow down if needed. This is useful for expanding
        /// stacks.
        const GROW_DOWN = 1 << 3;

        /// The area is permanent and cannot be unmapped. This is used for
        /// guarding the first and last page of the virtual memory, avoiding
        /// respectively valid null pointer dereference and preventing various
        /// attacks. This flag cannot be used by the user: any try to use it
        /// in a syscall will be denied.
        const PERMANENT = 1 << 4;
    }
}

impl From<PageEntryFlags> for Access {
    /// Convert a page entry flags into an access rights. Flags that are not
    /// related to access rights are ignored.
    fn from(flags: PageEntryFlags) -> Self {
        let mut access = Access::empty();
        if flags.contains(PageEntryFlags::PRESENT) {
            access |= Access::READ;
        }
        if flags.contains(PageEntryFlags::WRITABLE) {
            access |= Access::WRITE;
        }
        if !flags.contains(PageEntryFlags::NO_EXECUTE) {
            access |= Access::EXECUTE;
        }
        access
    }
}

impl From<PageFaultErrorCode> for Access {
    /// Convert a page fault error code into an access rights. It determine the
    /// type of access that caused the page fault. Due to the nature of the
    /// page fault error code, it should not possible to have multiple access
    /// rights at the same time, so we does not handle this case.
    ///
    /// Other flags in the page fault error code are ignored.
    fn from(error: PageFaultErrorCode) -> Self {
        let mut access = Access::empty();
        if error.contains(PageFaultErrorCode::WRITE_ACCESS) {
            access |= Access::WRITE;
        } else if error.contains(PageFaultErrorCode::INSTRUCTION_FETCH) {
            access |= Access::EXECUTE;
        } else {
            access |= Access::READ;
        }
        access
    }
}

impl From<Access> for PageEntryFlags {
    /// Convert an access rights into a page entry flags.
    fn from(access: Access) -> Self {
        let mut flags = PageEntryFlags::empty();
        if access.contains(Access::READ) {
            flags |= PageEntryFlags::PRESENT;
        }
        if access.contains(Access::WRITE) {
            flags |= PageEntryFlags::WRITABLE;
        }
        if !access.contains(Access::EXECUTE) {
            flags |= PageEntryFlags::NO_EXECUTE;
        }
        flags
    }
}
