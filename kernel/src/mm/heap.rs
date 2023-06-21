use super::frame::Frame;
use addr::Virtual;
use core::{
    alloc::{GlobalAlloc, Layout},
    ops::{Deref, Range},
};
use sync::Spinlock;

/// A heap that can be used for memory allocation. The inner heap is protected by a spinlock,
/// allowing for concurrent access to the heap.
pub struct Heap {
    inner: Spinlock<linked_list_allocator::Heap>,
}

impl Heap {
    /// Create a new heap with the given inner heap. This function does not initialise the heap,
    /// it is the responsibility of the caller to do so with the `init` function.
    #[must_use]
    pub const fn new(inner: linked_list_allocator::Heap) -> Self {
        Heap {
            inner: Spinlock::new(inner),
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
            usize::from(range.end.end() - range.start.addr()) * Frame::SIZE,
        );
    }
}

impl Deref for Heap {
    type Target = Spinlock<linked_list_allocator::Heap>;
    fn deref(&self) -> &Spinlock<linked_list_allocator::Heap> {
        &self.inner
    }
}

unsafe impl GlobalAlloc for Heap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.inner
            .lock()
            .allocate_first_fit(layout)
            .ok()
            .map_or(core::ptr::null_mut(), core::ptr::NonNull::as_ptr)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner
            .lock()
            .deallocate(core::ptr::NonNull::new_unchecked(ptr), layout);
    }
}
