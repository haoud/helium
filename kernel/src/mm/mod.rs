use self::heap::Heap;
use crate::limine::LIMINE_MEMMAP;
use frame::{
    allocator::{dummy, Allocator},
    AllocationFlags,
};

pub mod frame;
pub mod heap;

/// The heap allocator used by the kernel
#[global_allocator]
static HEAP_ALLOCATOR: Heap = Heap::new();

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
