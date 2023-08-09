use self::heap::Heap;
use crate::x86_64::paging::PAGE_SIZE;
use addr::{phys::Physical, virt::Virtual};
use core::ops::Range;
use frame::{
    allocator::{dummy, Allocator},
    AllocationFlags,
};
use limine::{LimineHhdmRequest, LimineMemmapRequest};
use macros::init;
use sync::{Lazy, Spinlock};

pub mod frame;
pub mod heap;
pub mod vmm;

/// The request to the limine bootloader to get a memory map.
pub static LIMINE_MEMMAP: LimineMemmapRequest = LimineMemmapRequest::new(0);

/// The request to the limine bootloader to get a HHDM, mapping all the physical memory at a
/// specific address (`0xFFFF_8000_0000_0000`).
pub static LIMINE_HHDM: LimineHhdmRequest = LimineHhdmRequest::new(0);

/// The heap allocator used by the kernel
#[global_allocator]
static HEAP_ALLOCATOR: Heap = Heap::new(linked_list_allocator::Heap::empty());

/// The frame allocator used by the kernel.
pub static FRAME_ALLOCATOR: Lazy<Spinlock<dummy::Allocator>> = Lazy::new(|| {
    let mmap = LIMINE_MEMMAP
        .get_response()
        .get()
        .expect("No memory map found")
        .memmap();

    Spinlock::new(dummy::Allocator::new(mmap))
});

/// The number of pages allocated for the heap (16 MiB).
const HEAP_PAGE_COUNT: usize = 4096;

/// Represent a conveniant way to store a number of pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageCount(pub usize);

impl PageCount {
    /// Creates a new `PageCount` from a number of pages.
    #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }

    /// Creates a new `PageCount` from a number of bytes. If the number of bytes
    /// is not a multiple of the page size, the number of pages is rounded down
    /// to the nearest page aligned number of bytes.
    #[must_use]
    pub const fn from_size_truncate(bytes: usize) -> Self {
        Self(bytes / PAGE_SIZE)
    }

    /// Creates a new `PageCount` from a number of bytes. If the number of bytes
    /// is not a multiple of the page size, the number of pages is rounded up
    /// to the nearest page aligned number of bytes.
    #[must_use]
    pub const fn from_size_extend(bytes: usize) -> Self {
        Self((bytes + PAGE_SIZE - 1) / PAGE_SIZE)
    }
}

impl From<PageCount> for usize {
    fn from(count: PageCount) -> Self {
        count.0
    }
}

impl From<usize> for PageCount {
    fn from(count: usize) -> Self {
        assert!(count % PAGE_SIZE == 0);
        Self(count / PAGE_SIZE)
    }
}

impl From<Range<Virtual>> for PageCount {
    fn from(range: Range<Virtual>) -> Self {
        assert!(range.start.is_page_aligned());
        assert!(range.end.is_page_aligned());
        Self(usize::from(range.end - range.start) / PAGE_SIZE)
    }
}

impl From<Range<Physical>> for PageCount {
    fn from(range: Range<Physical>) -> Self {
        assert!(range.start.is_page_aligned());
        assert!(range.end.is_page_aligned());
        Self(usize::from(range.end - range.start) / PAGE_SIZE)
    }
}

/// Initializes the memory subsystem. It parses the memory map and fills the frame array in order
/// to allow the allocation of physical memory frames, and then allocate a range of frames for the
/// heap.
///
/// # Safety
/// This function is unsafe because it use unsafe code and raw pointers to initialize the
/// memory subsystem.
#[init]
pub unsafe fn setup() {
    let frames = FRAME_ALLOCATOR
        .lock()
        .allocate_range(HEAP_PAGE_COUNT, AllocationFlags::KERNEL)
        .expect("Failed to allocate memory for the heap")
        .into_inner();

    HEAP_ALLOCATOR.init(frames);
}
