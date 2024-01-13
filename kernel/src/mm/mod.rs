use crate::{limine::LIMINE_MEMMAP, module};
use self::heap::Heap;
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

/// Reclaim the memory used by the kernel during the boot process.
/// 
/// # Safety
/// This function is unsafe because it will free the memory used by the kernel
/// during the boot process. Trying to use the memory after calling this function
/// will cause undefined behavior: the caller must ensure that there is no more
/// references to the memory that will be freed.
pub unsafe fn reclaim_boot_memory() {
    // Compute the number of memory that cound be freed
    let mut size = FRAME_ALLOCATOR
        .lock()
        .state
        .reclaim_boot_memory()
        .iter()
        .map(|range| range.end.addr().as_usize() - range.start.addr().as_usize())
        .sum::<usize>();

    // TODO: Reclaim the memory used by .init section

    // The shell was copied into the ramfs and isn't needed anymore
    // The init task was loaded into memory and isn't needed anymore
    size += module::free("/boot/shell.elf").unwrap_or_default();
    size += module::free("/boot/init.elf").unwrap_or_default();
    log::info!("{size} bytes of boot memory was reclaimed by the kernel");
}
