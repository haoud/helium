use self::heap::Heap;
use frame::{
    allocator::{dummy, Allocator},
    AllocationFlags,
};
use limine::{LimineHhdmRequest, LimineMemmapRequest};
use macros::init;
use sync::{Lazy, Spinlock};

pub mod cache;
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
