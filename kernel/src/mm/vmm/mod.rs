use self::area::{Access, Area, Flags, Type};
use super::{
    frame::{allocator::Allocator, AllocationFlags},
    FRAME_ALLOCATOR,
};
use crate::x86_64::paging::{self, MapError, PageEntryFlags, PageTableRoot, PAGE_SIZE};
use addr::{user::UserVirtual, virt::Virtual};
use alloc::collections::BTreeMap;
use core::ops::Range;
use lib::align::Align;

pub mod area;

pub struct Manager {
    areas: Option<BTreeMap<UserVirtual, Area>>,
    table: PageTableRoot,
}

impl Manager {
    /// Create a new area manager. This manager contains two areas, both of which
    /// are permanent and one page long. The first area is mapped at the beginning
    /// of the virtual address space and the second area is mapped at the end of
    /// the virtual address space.
    ///
    /// These two areas are used to prevent the user to dereference a null pointer
    /// a grant them a valid address to dereference, which would likely be a bug.
    /// It also prevents various attacks from the user that imply dereferencing
    /// null pointers or allocating memory just before the start of the canonical
    /// hole (for exemple, the SYSRET bug in Intel processors)
    ///
    /// It also allows some optimizations in the kernel and make some things easier,
    /// for example when trying to find a free range of virtual addresses.
    #[must_use]
    pub fn new() -> Self {
        let mut areas = BTreeMap::new();
        let table = PageTableRoot::new();

        // The "null pointer dereference" guard
        let start = Area::builder()
            .range(UserVirtual::zero()..UserVirtual::new(PAGE_SIZE))
            .flags(Flags::PERMANENT)
            .access(Access::empty())
            .kind(Type::Anonymous)
            .build();

        // The "end-of-the-world-and-hardware-vulnerability" guard
        let end = Area::builder()
            .range(UserVirtual::last_page_aligned()..UserVirtual::last())
            .flags(Flags::PERMANENT)
            .access(Access::empty())
            .kind(Type::Anonymous)
            .build();

        areas.insert(start.base(), start);
        areas.insert(end.base(), end);
        Self {
            areas: Some(areas),
            table,
        }
    }

    /// Create a new area manager for kernel tasks. This manager does not contain
    /// any area and is only used to have access to the kernel page table.
    #[must_use]
    pub fn kernel() -> Self {
        Self {
            areas: None,
            table: PageTableRoot::new(),
        }
    }

    /// Map a area of virtual memory. The memory is not allocated until it is accessed
    /// by the user (lazy allocation), this function simply reserve the virtual memory
    /// to avoid that the user maps multiple areas at the same location.
    ///
    /// # Errors
    /// If the area was sucdessfully mapped, then this function returns the range
    /// of virtual addresses that were mapped, that can be different from the specified
    /// range if the FIXED flag was not set.
    /// Otherwise, this function can return the following errors:
    /// - `InvalidRange`: the range is not page aligned or has a length of zero
    /// - `InvalidFlags`: the area has the `PERMANENT flag`: this flag is only for
    ///                   kernel use and for specific areas.
    /// - `WouldOverlap`: the area overlaps with an existing area and the FIXED flag
    ///                   was set.
    /// - `OutOfVirtualMemory`: there is not enough contiguous virtual memory to map
    ///                         the area.
    ///
    /// # Panics
    /// Panics if the manager does not contain a area map. This mean that this is a
    /// manager for kernel task, where mapping user memory is not allowed.
    pub fn mmap(&mut self, mut area: Area) -> Result<Range<UserVirtual>, MmapError> {
        // If the area is not page aligned or has a length of zero, then return
        // an error because it is invalid.
        if !Virtual::from(area.base()).is_page_aligned() || area.is_empty() {
            return Err(MmapError::InvalidRange);
        }

        // If the area has the permanent flag, then return an error because it
        // is only for kernel use.
        if area.flags().contains(Flags::PERMANENT) {
            return Err(MmapError::InvalidFlags);
        }

        // If the area does not have a defined range or overlaps with an existing
        // area, then try to find a free range for it. If no free range is found,
        // then return an error.
        if area.base().is_null() || self.overlaps(area.range()) {
            if area.flags().contains(area::Flags::FIXED) {
                return Err(MmapError::WouldOverlap);
            }

            let range = self
                .find_free_range(area.len())
                .ok_or(MmapError::OutOfVirtualMemory)?;
            area.set_range(range);
        }

        // Insert the area into the map and return the range that it occupies.
        let range = area.range().clone();
        self.areas.as_mut().unwrap().insert(area.base(), area);
        Ok(range)
    }

    /// Map an page of virtual memory that the user want to access. Since the `mmap`
    /// function performs a lazy allocation, accessing any page in the returned range
    /// will cause a page fault. This function simply maps the page that the user want
    /// to access, performing some checks to ensure that the user has the right to
    /// access the page.
    ///
    /// # Errors
    /// - `AccessDenied`: the user attempted to access a page with an access
    ///                   right that there is not allowed by the area.
    /// - `OutOfMemory`: the kernel was unable to allocate a new frame.
    /// - `NotMapped`: the given address is not contained in any area.
    ///
    /// # Panics
    /// This function panics if the given address is already mapped. This is a
    /// kernel bug and should never happen and must be fixed.
    pub fn page_in(&mut self, address: UserVirtual, access: Access) -> Result<(), PageInError> {
        // Find the area that contains the given address
        let area = self.find_area(address).ok_or(PageInError::NotMapped)?;

        // If the area does not have the requested access, then return an
        // error: the user tried to access a page that they do not have
        // permission to access.
        if !area.access().contains(access) {
            return Err(PageInError::AccessDenied);
        }

        // Depending on the type of the area, we need to handle the page in
        // differently. For now, we only support anonymous areas and we simply
        // allocate a new zeroed frame and map it at the given address.
        match area.kind() {
            Type::Anonymous => unsafe {
                let frame = FRAME_ALLOCATOR
                    .lock()
                    .allocate_frame(AllocationFlags::ZEROED)
                    .ok_or(PageInError::OutOfMemory)?;

                let flags = PageEntryFlags::from(area.access()) | PageEntryFlags::USER;
                let virt = Virtual::from(address);

                paging::map(&self.table, virt, frame, flags)?;
            },
        }

        Ok(())
    }

    /// Find a free range of virtual addresses that can contain the given size. If
    /// no free range is found, then this function returns None.
    fn find_free_range(&self, size: usize) -> Option<Range<UserVirtual>> {
        let area = self.areas.as_ref().unwrap();
        area.iter()
            .zip(area.iter().skip(1))
            .find_map(|((_, area), (_, next))| {
                let start = usize::from(area.range().end).align_up(PAGE_SIZE);
                let end = usize::from(next.range().start);
                if end.saturating_sub(start) >= size {
                    let end = UserVirtual::new(start + size);
                    let start = UserVirtual::new(start);
                    Some(start..end)
                } else {
                    None
                }
            })
    }

    /// Verify if the range overlaps with any existing area.
    fn overlaps(&self, range: &Range<UserVirtual>) -> bool {
        self.areas
            .as_ref()
            .unwrap()
            .range(..=range.end)
            .next_back()
            .map_or(false, |(_, area)| area.range().end > range.start)
    }

    /// Find the area that contains the given address. If no area contains the
    /// given address, then this function returns None.
    fn find_area(&self, address: UserVirtual) -> Option<&Area> {
        self.areas
            .as_ref()
            .unwrap()
            .range(..=address)
            .next_back()
            .map(|(_, area)| area)
            .filter(|area| area.range().contains(&address))
    }

    /// Return a reference to the page table of this manager.
    pub fn table(&self) -> &PageTableRoot {
        &self.table
    }
}

impl Default for Manager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MmapError {
    InvalidFlags,
    InvalidRange,
    WouldOverlap,
    OutOfVirtualMemory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnmapError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MprotectError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageInError {
    NotMapped,
    OutOfMemory,
    AccessDenied,
}

impl From<MapError> for PageInError {
    fn from(e: MapError) -> Self {
        match e {
            MapError::OutOfMemory => Self::OutOfMemory,
            MapError::AlreadyMapped => panic!("User page already mapped"),
        }
    }
}
