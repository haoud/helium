use addr::{frame::Frame, virt::Virtual};
use core::{
    alloc::{GlobalAlloc, Layout},
    ops::{Deref, Range}, sync::atomic::{AtomicUsize, Ordering},
};

/// A heap that can be used for memory allocation. The inner heap is protected by a spinlock,
/// allowing for concurrent access to the heap.
pub struct Heap {
    inner: Spinlock<linked_list_allocator::Heap>,
    allocated: AtomicUsize,
}

impl Heap {
    /// Create a new heap. This function does not initialise the heap,
    /// it is the responsibility of the caller to do so with the `init` function.
    #[must_use]
    pub const fn new() -> Self {
        Heap {
            inner: Spinlock::new(linked_list_allocator::Heap::empty()),
            allocated: AtomicUsize::new(0),
        }
    }

    /// Initialise the heap with the given range of frames.
    ///
    /// # Safety
    /// This function is unsafe because this function should only be called once and only with a
    /// empty heap. The range of frame must have be allocated before calling this function and must
    /// stay allocated until the end of the scope of this object.
    pub unsafe fn init(&self, range: Range<Frame>) {
        self.inner.lock().init(
            Virtual::from(range.start.addr()).as_mut_ptr::<u8>(),
            usize::from(range.end.addr() - range.start.addr()),
        );
    }
}

impl Deref for Heap {
    type Target = Spinlock<linked_list_allocator::Heap>;
    fn deref(&self) -> &Spinlock<linked_list_allocator::Heap> {
        &self.inner
    }
}

/// Implement the global allocator trait for the heap. This allows the heap to be used as the
/// default allocator, enabling the use of the `alloc` crate like an (almost) normal program.
unsafe impl GlobalAlloc for Heap {
    /// Allocate memory with the given layout. This function returns a null pointer if the
    /// allocation failed.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocated.fetch_add(layout.size(), Ordering::SeqCst);
        self.inner
            .lock()
            .allocate_first_fit(layout)
            .ok()
            .map_or(core::ptr::null_mut(), core::ptr::NonNull::as_ptr)
    }

    /// Deallocate the memory at the given pointer with the given layout. This function does
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner
            .lock()
            .deallocate(core::ptr::NonNull::new_unchecked(ptr), layout);
        self.allocated.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}
