use self::table::{FetchBehavior, PageEntry, PageEntryFlags, PageTable, PageTableRoot};
use super::cpu;
use crate::{
    mm::{
        frame::{allocator::Allocator, AllocationFlags},
        FRAME_ALLOCATOR,
    },
    user::scheduler::{Scheduler, SCHEDULER},
    user::vmm::area::Access,
    x86_64::paging::table::PageFaultErrorCode,
};
use addr::{frame::Frame, phys::Physical, user::UserVirtual, virt::Virtual};
use macros::init;
use sync::Lazy;

pub mod table;
pub mod tlb;

/// The size of a page. This is always 4096 bytes, Helium does not support 2 MiB or 1 GiB pages
/// to keep the code simple.
pub const PAGE_SIZE: usize = 4096;

/// The kernel page table. This is used to create new page table very fast, simply by copying the
/// kernel page table into the new page table. This page table is also shared between all kernel
/// threads, so that we do not need to create a new page table for each kernel thread, and mapping
/// or unmapping a page in the kernel space is serialized by the page table lock to avoid data
/// races.
pub static KERNEL_PML4: Lazy<PageTableRoot> =
    Lazy::new(|| unsafe { PageTableRoot::from_page(cpu::Cr3::address()) });

/// Represents a level inside the page table hierarchy. This is used to traverse the page table
/// hierarchy.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Level {
    Pml4,
    Pdpt,
    Pd,
    Pt,
}

impl Level {
    /// Get the next level in the page table hierarchy from this level. If this is the last level
    /// (`Level::Pt`), this function returns `None`.
    #[must_use]
    pub fn next(self) -> Option<Self> {
        match self {
            Self::Pml4 => Some(Self::Pdpt),
            Self::Pdpt => Some(Self::Pd),
            Self::Pd => Some(Self::Pt),
            Self::Pt => None,
        }
    }

    /// Return the index of this level in the page table hierarchy. The PML4 is the highest level
    /// and has index 4, the PT is the lowest level and has index 1.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Pml4 => 4,
            Self::Pdpt => 3,
            Self::Pd => 2,
            Self::Pt => 1,
        }
    }
}

/// Initializes the paging system. This function must be called before any other function in this
/// module. This function initialize the kernel page table with the current page table, and clears
/// the first 256 entries in the PML4 table, which are reserved for the user space.
///
/// We also preallocate all the kernel PML4 entries, so that when we create a new address space,
/// we can simply copy the kernel PML4 entries to the new PML4 table without worrying when a kernel
/// PML4 entry will be allocated in a address space (which would need synchronization between all
/// address spaces).
///
/// # Safety
/// This function is unsafe because the caller must ensure that it is called only once, and before
/// any other function in this module. The caller must also ensure that the current page table will
/// remain valid for the lifetime of the kernel.
///
/// # Panics
/// This function will panic if the kernel ran out of memory while preallocating the kernel page
/// tables.
#[init]
pub unsafe fn setup() {
    let mut pml4 = KERNEL_PML4.lock();

    pml4.clear_userspace();
    pml4.kernel_space_mut()
        .iter_mut()
        .filter(|entry| !entry.present())
        .for_each(|entry| {
            let flags = PageEntryFlags::PRESENT | PageEntryFlags::WRITABLE;
            let frame = FRAME_ALLOCATOR
                .lock()
                .allocate_frame(AllocationFlags::KERNEL | AllocationFlags::ZEROED)
                .expect("Out of memory while preallocating kernel page tables")
                .into_inner();

            entry.set_address(frame.addr());
            entry.set_flags(flags);
        });
}

/// Map a frame at the specified virtual address. If the address is already mapped, an error is
/// returned.
///
/// # Errors
/// If the frame cannot be mapped at the specified address, an `MapError` is returned, containing
/// the reason why the frame cannot be mapped.
///
/// # Safety
/// This function is unsafe because the caller must ensure that the frame will remain free until
/// the page is unmapped. The caller must also ensure that the mapping does not break the memory
/// safety of the kernel.
pub unsafe fn map(
    root: &PageTableRoot,
    address: Virtual,
    frame: Frame,
    flags: PageEntryFlags,
) -> Result<(), MapError> {
    let mut table = root.lock();
    let pte = table
        .fetch_last_entry(address, FetchBehavior::Allocate)
        .map_err(|_| MapError::OutOfMemory)?;

    if let Some(addr) = pte.address() {
        log::warn!("Attempt to map an already mapped page (frame: {})", addr);
        Err(MapError::AlreadyMapped)
    } else {
        pte.set_flags(PageEntryFlags::PRESENT | flags);
        pte.set_address(frame.addr());
        tlb::shootdown(address);
        Ok(())
    }
}

/// Unmap the page mapped at the specified address. If an entry is the hierarchy is not present,
/// we return an error, otherwise we clear the entry, flush the TLB on all CPUs, and return the
/// previously mapped physical frame. It is the responsibility of the caller to free the returned
/// frame.
///
/// # Errors
/// If the address is not mapped, an `UnmapError` is returned, describing the error. Otherwise,
/// the function returns the previously mapped frame. The caller is responsible for freeing the
/// frame if needed.
///
/// # Safety
/// This function is unsafe because the caller must ensure that the virtual address will not be
/// used after the function returns. The caller must also ensure that the frame returned by this
/// frame is correctly freed if allocated with the frame allocator.
pub unsafe fn unmap(root: &PageTableRoot, address: Virtual) -> Result<Frame, UnmapError> {
    let mut table = root.lock();
    let pte = table
        .fetch_last_entry(address, FetchBehavior::Reach)
        .map_err(|_| UnmapError::NotMapped)?;

    if let Some(physical) = pte.address() {
        pte.clear();
        tlb::shootdown(address);
        return Ok(Frame::new(physical));
    }

    Err(UnmapError::NotMapped)
}

/// Resolve a virtual address to a physical address. If the address is not mapped, return `None`.
/// This function allow the address passed as argument to not be page aligned, but will always
/// return a page aligned physical address.
pub fn resolve(root: &PageTableRoot, address: Virtual) -> Option<Physical> {
    unsafe {
        root.lock()
            .fetch_last_entry(address, FetchBehavior::Reach)
            .map_or(None, |pte| pte.address())
    }
}

/// Deallocates all frames recursively in a page table and frees the page table itself. We take
/// as parameter a slice of page entries to allow the caller to only deallocate recursively a
/// subset of a page table.
///
/// Even if only an subset of the page table is specified, the page table will be still
/// deallocated by the function: specify an subset of the page table only prevents the function
/// from deallocating the frames recursively in the non specified part of the page table (see
/// the `Drop` implementation of `PageTableRoot` for more details).
///
/// # Safety
/// This function is unsafe because the caller must ensure that the page table is not used
/// after this function is called.
unsafe fn deallocate_recursive(table: &mut [PageEntry], level: Level) {
    table
        .iter()
        .filter_map(PageEntry::address)
        .for_each(|address| match level {
            Level::Pml4 | Level::Pdpt | Level::Pd => {
                let table = PageTable::from_page_mut(Virtual::from(address));
                deallocate_recursive(table, level.next().unwrap());
            }
            Level::Pt => {
                FRAME_ALLOCATOR.lock().deallocate_frame(Frame::new(address));
            }
        });

    let virt = Virtual::from_ptr(table.as_mut_ptr());
    let phys = Physical::from(virt.page_align_down());
    FRAME_ALLOCATOR.lock().deallocate_frame(Frame::new(phys));
}

/// Handle a page fault. This function is called by the page fault handler when a page fault
/// occurs. For now, a page fault that concerned a kernel address is considered unrecoverable,
/// and will panic. If the page fault concerned a user address, we try to page in the page if
/// the page is not present in memory. If the page was successfully paged in, we can return
/// from the page fault handler, otherwise we panic.
///
/// # Panics
/// This function panics if the page fault is not recoverable.
pub fn handle_page_fault(addr: Virtual, code: PageFaultErrorCode) {
    let present = code.contains(PageFaultErrorCode::PRESENT);

    if let Ok(uaddr) = UserVirtual::try_new(addr.as_usize()) {
        // Try to page in the page if it is not present in memory. If the page
        // was successfully paged in, we can return immediately, otherwise the
        // page fault is unrecoverable.
        if let Some(table) = SCHEDULER.current_task().thread().lock().vmm() {
            if !present && table.lock().page_in(uaddr, Access::from(code)).is_ok() {
                return;
            }
        } else {
            panic!("Page fault in kernel thread");
        }
    }

    panic!("Page fault exception at {:#x}", addr);
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MapError {
    /// The kernel ran out of memory while trying to allocate a new page table
    OutOfMemory,

    /// The virtual address was already mapped to a physical address
    AlreadyMapped,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnmapError {
    /// The virtual address was not mapped to a physical address
    NotMapped,
}
