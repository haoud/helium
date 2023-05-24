#![no_std]
#![feature(step_trait)]
#![feature(const_mut_refs)]

use frame::{
    allocator::{dummy, Allocator},
    AllocationFlags,
};
use heap::LockedHeap;
use limine::{LimineHhdmRequest, LimineMemmapRequest};
use macros::init;
use sync::Spinlock;

pub mod frame;
pub mod heap;

/// The request to the limine bootloader to get a memory map.
pub static LIMINE_MEMMAP: LimineMemmapRequest = LimineMemmapRequest::new(0);

/// The request to the limine bootloader to get a HHDM, mapping all the physical memory at a
/// specific address (0xFFFF_8000_0000_0000).
pub static LIMINE_HHDM: LimineHhdmRequest = LimineHhdmRequest::new(0);

/// The frame allocator used by the kernel.
pub static FRAME_ALLOCATOR: Spinlock<dummy::Allocator> =
    Spinlock::new(dummy::Allocator::uninitialized());

/// The heap allocator used by the kernel
#[global_allocator]
static HEAP_ALLOCATOR: LockedHeap = LockedHeap::new(linked_list_allocator::Heap::empty());

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
    let mmap = LIMINE_MEMMAP
        .get_response()
        .get()
        .expect("No memory map found")
        .memmap();

    *FRAME_ALLOCATOR.lock() = dummy::Allocator::new(mmap);

    let frames = FRAME_ALLOCATOR
        .lock()
        .allocate_range(HEAP_PAGE_COUNT, AllocationFlags::KERNEL)
        .expect("Failed to allocate heap");
    HEAP_ALLOCATOR.init(frames);
}
