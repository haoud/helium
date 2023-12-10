use self::area::{Access, Area, Flags, Type};
use crate::mm::{
    frame::{allocator::Allocator, AllocationFlags},
    FRAME_ALLOCATOR,
};
use crate::x86_64::paging::{self, MapError, PageEntryFlags, PageTableRoot, PAGE_SIZE};
use addr::{user::UserVirtual, virt::Virtual};
use alloc::{collections::BTreeMap, vec::Vec};
use core::ops::Range;

pub mod area;

/// The virtual memory manager. This manager is responsible for mapping and
/// unmapping virtual memory for the user. It also contains a page table that
/// is used to map user memory.
/// This manager can also be used for kernel tasks, but it does not contain
/// any area and is only used to have access to the kernel page table and
/// have a common interface with user tasks.
#[derive(Debug)]
pub struct Manager {
    areas: Option<BTreeMap<UserVirtual, Area>>,
    table: PageTableRoot,
}

impl Manager {
    /// Create a new area manager. This manager contains two areas, both of which
    /// are permanent and one page long. The first area is mapped at the beginning
    /// of the virtual address space and the second area is mapped at the end of
    /// the virtual address space (almost at the end, since the last page is not
    /// even considered mappable by the manager)
    ///
    /// These two areas are used to prevent the user to dereference a null pointer
    /// a grant them a valid address to dereference, which would likely be a bug.
    /// It also prevents various attacks from the user that imply dereferencing
    /// null pointers or allocating memory just before the start of the canonical
    /// hole (for exemple, the SYSRET bug in Intel processors)
    ///
    /// It also allows some optimizations in the kernel and make some things easier,
    /// for example when trying to find a free range of virtual addresses or when
    /// page aligning an user virtual address.
    #[must_use]
    pub fn new() -> Self {
        let mut areas = BTreeMap::new();
        let table = PageTableRoot::new();

        // The "null pointer dereference" guard
        let start = UserVirtual::zero();
        let end = UserVirtual::new(PAGE_SIZE);
        let null_guard = Area::builder()
            .flags(Flags::PERMANENT)
            .access(Access::empty())
            .kind(Type::Anonymous)
            .range(start..end)
            .build();

        // The "end of the world" guard
        let start = UserVirtual::second_last_aligned_page();
        let end = UserVirtual::last_aligned_page();
        let end_guard = Area::builder()
            .flags(Flags::PERMANENT)
            .access(Access::empty())
            .kind(Type::Anonymous)
            .range(start..end)
            .build();

        areas.insert(null_guard.base(), null_guard);
        areas.insert(end_guard.base(), end_guard);
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
    /// - `InvalidRange`: the range is not page aligned, has a length of zero or has an end
    ///                   address that is greater than `UserVirtual::second_last_page_aligned()`.
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
        if !valid_range(area.range()) {
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
        if area.base().is_null() || self.overlaps_with_existing(area.range()) {
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
        self.insert_area(area);
        Ok(range)
    }

    /// Unmap a range of virtual memory. This function will unmap all the mappings
    /// contained in the given range (aligned to the page size). If there is no
    /// mapping in the given range, then this function does nothing.
    ///
    /// # Errors
    /// If the range was successfully unmapped, then this function returns `Ok(())`.
    /// Otherwise, this function can return the following errors:
    /// - `InvalidRange`: the range is not page aligned, has a length of zero or has an end
    ///                  address that is greater than `UserVirtual::second_last_page_aligned()`.
    ///
    /// # Panics
    /// Panics if the manager does not contain a area map. This mean that this is a
    /// manager for kernel task, where mapping user memory is not allowed. A panic
    /// can also occur if the code that unmap the range is not correct and is
    /// detected by a few assertions.
    pub fn munmap(&mut self, range: Range<UserVirtual>) -> Result<(), UnmapError> {
        if !valid_range(&range) {
            return Err(UnmapError::InvalidRange);
        }

        let range_end_aligned = range.end.page_align_up();
        let range_start = range.start;

        // Drain and collect all the areas that collide with the given range.
        let areas = self
            .areas
            .as_mut()
            .unwrap()
            .extract_if(|_, area| {
                range_overlaps(&range, area.range())
                    || range_contains(&range, area.range())
                    || range_contained(&range, area.range())
            })
            .map(|(_, area)| area)
            .collect::<Vec<_>>();

        // Process each area that we collected for unmapping. Depending on how
        // the range to unmap overlaps with the area, those areas will be deleted,
        // modified or split into two areas.
        for mut area in areas {
            let area_start = area.range().start;
            let area_end = area.range().end;

            let unmap_range = if range_contains(&range, area.range()) {
                // The area contains the range, so we need to split the area
                // into two areas, one before the range and one after the range.
                // Then we insert the two areas into the map and return the
                // range that we need to unmap.
                let mut split = area.clone();

                area.set_range(area_start..range_start);
                split.set_range(range_end_aligned..area_end);

                self.insert_area(split);
                self.insert_area(area);
                range.clone()
            } else if range_contained(&range, area.range()) {
                // The area is fully contained in the range, so we can simply
                // unmap it
                area.range().clone()
            } else if range_overlaps(&range, area.range()) {
                // The area overlaps with the range, so we need first need to
                // change if the range overlaps with the start or the end of
                // the area to change accordingly the range of the area, and
                // then return the range that we need to unmap.
                let range = if range.end > area.range().start {
                    // Unmap the start of the area
                    area.set_range(range_end_aligned..area_end);
                    area_start..range_end_aligned
                } else if range.start < area.range().end {
                    // Unmap the end of the area
                    area.set_range(area_start..range_start);
                    range_start..area_end
                } else {
                    unreachable!("Unmap: overlap algorithm implementation error");
                };

                self.insert_area(area);
                range
            } else {
                unreachable!("Unmap: algorithm implementation error");
            };

            self.unmap_range(unmap_range);
        }

        Ok(())
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
                    .ok_or(PageInError::OutOfMemory)?
                    .into_inner();

                let flags = PageEntryFlags::from(area.access()) | PageEntryFlags::USER;
                let virt = Virtual::from(address);

                paging::map(&self.table, virt, frame, flags)?;
            },
        }

        Ok(())
    }

    /// Unmap the range of user virtual addresses and deallocate the frames that
    /// were mapped at these addresses.
    fn unmap_range(&mut self, range: Range<UserVirtual>) {
        range.step_by(PAGE_SIZE).for_each(|address| unsafe {
            if let Ok(frame) = paging::unmap(&self.table, Virtual::from(address)) {
                FRAME_ALLOCATOR.lock().deallocate_frame(frame);
            }
        });
    }

    /// Find a free range of virtual addresses that can contain the given size. If
    /// no free range is found, then this function returns None.
    fn find_free_range(&self, size: usize) -> Option<Range<UserVirtual>> {
        let area = self.areas.as_ref().unwrap();
        area.iter()
            .zip(area.iter().skip(1))
            .find_map(|((_, area), (_, next))| {
                let start = usize::from(area.range().end.page_align_up());
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
    fn overlaps_with_existing(&self, range: &Range<UserVirtual>) -> bool {
        self.areas
            .as_ref()
            .unwrap()
            .range(..range.end)
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

    /// Insert a new area into the manager
    fn insert_area(&mut self, area: Area) {
        self.areas.as_mut().unwrap().insert(area.base(), area);
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
pub enum UnmapError {
    InvalidRange,
}

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

/// Return true if the two ranges overlap with each other. If the end of the
/// range is not page aligned, then it is aligned up.
fn range_overlaps(a: &Range<UserVirtual>, b: &Range<UserVirtual>) -> bool {
    let a_end = usize::from(a.end.page_align_up());
    let b_end = usize::from(b.end.page_align_up());
    let a_start = usize::from(a.start);
    let b_start = usize::from(b.start);

    a_start < b_end && a_end > b_start
}

/// Return true if the first range contains the second range. If they share
/// a border, then this function returns false. If the end of the range is
/// not page aligned, then it is aligned up before the comparison.
fn range_contains(a: &Range<UserVirtual>, b: &Range<UserVirtual>) -> bool {
    let a_end = usize::from(a.end.page_align_up());
    let b_end = usize::from(b.end.page_align_up());
    let a_start = usize::from(a.start);
    let b_start = usize::from(b.start);

    a_start < b_start && a_end > b_end
}

/// Return true if the first range is contained in the second range. If they share
/// a border, then this function still returns true. If the end of the range is
/// not page aligned, then it is aligned up before the comparison.
fn range_contained(a: &Range<UserVirtual>, b: &Range<UserVirtual>) -> bool {
    let a_end = usize::from(a.end.page_align_up());
    let b_end = usize::from(b.end.page_align_up());
    let a_start = usize::from(a.start);
    let b_start = usize::from(b.start);

    a_start >= b_start && a_end <= b_end
}

/// Verify if the given range is valid. This function reject any range that :
/// - Include any address equal or beyond `UserVirtual::last_page_aligned()`
/// - Has not a page aligned start
/// - Has a length of zero
///
/// # Explanation
/// - Rejecting any address equal or beyond `UserVirtual::last_page_aligned()` is
///   a security measure to prevent the user to map the last page of the virtual
///   address space, involved in some attacks and hardware bugs.
///   Furthermore, this allow some optimizations: it is possible to safely page
///   align-up any validated user virtual addresses without checking if they are
///   still in user space and not in the canonical hole.
fn valid_range(range: &Range<UserVirtual>) -> bool {
    Virtual::from(range.start).is_page_aligned()
        && range.end <= UserVirtual::last_aligned_page()
        && !range.is_empty()
}
