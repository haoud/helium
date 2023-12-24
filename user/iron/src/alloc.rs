use crate::syscall;
use talc::*;

/// The minimum size of a heap in bytes. When the heap is created or extended, it will be
/// at least this size to avoid too much overhead and fragmentation.
const MINIMUM_HEAP_SIZE: usize = 1024 * 1024;

/// The allocator provided by Iron. It is a wrapper around the Talc allocator which is
/// performant and very convenient to use since it allows for multiple heaps and a custom
/// out-of-memory handler.
pub struct Allocator {
    talc: Talck<spin::Mutex<()>, ClaimHandler>,
}

impl Allocator {
    /// Creates a new empty allocator. Memory will automatically be claimed during the first
    /// allocation or if all the heaps are full.
    pub const fn empty() -> Self {
        Self {
            talc: Talc::new(ClaimHandler).lock(),
        }
    }
}

unsafe impl core::alloc::GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.talc.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.talc.dealloc(ptr, layout)
    }

    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.talc.alloc_zeroed(layout)
    }

    unsafe fn realloc(
        &self,
        ptr: *mut u8,
        layout: core::alloc::Layout,
        new_size: usize,
    ) -> *mut u8 {
        self.talc.realloc(ptr, layout, new_size)
    }
}

struct ClaimHandler;
impl OomHandler for ClaimHandler {
    fn handle_oom(talc: &mut Talc<Self>, layout: core::alloc::Layout) -> Result<(), ()> {
        // Compute the requested size of the allocation, aligned to the next page boundary and
        // then compute the size of the heap, which is the minimum between the requested size
        // and the minimum heap size defined above.
        let requested_size = (layout.size() + 4095) & !0xFFF;
        let allocated_size = core::cmp::max(requested_size * 2, MINIMUM_HEAP_SIZE);

        // Request a new memory region from the kernel. The kernel will automatically find a
        // free region of memory inside the process address space and return it to us.
        //
        // HOWEVER, even if the syscall succeeds, this does not mean that all the region is
        // already reserved for us. The kernel may have returned a region but without mapping
        // it to the process address space (demand paging), and can fail to do so later and the
        // programm will be simply terminated.
        //
        // Maybe we should add a flag to the syscall to tell the kernel to map the region
        // immediately ?
        //
        // SAFETY: This is safe because we ensure that Rust memory safety is not broken while
        // using the memory region returned by the kernel.
        let heap = unsafe {
            syscall::mmu::map(
                0,
                allocated_size,
                syscall::mmu::Access::READ_WRITE,
                syscall::mmu::Flags::PRIVATE,
            )
            .map_err(|_| ())?
        };

        // SAFETY: This is safe because the memory provided to the allocator, conforming to the
        // 'claim' method, is guaranteed to be valid, accessible to read and write accesses and
        // not overlapping with any other memory region. It is also guaranteed to not be mutated
        // while the allocator is using it because we have allocated a private memory region for
        // the allocator.
        unsafe {
            let start = heap as *mut u8;
            let end = (heap + allocated_size) as *mut u8;
            talc.claim(Span::new(start, end)).map_err(|_| ())?;
        }

        Ok(())
    }
}
