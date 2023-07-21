use crate::mm::{
    frame::{allocator::Allocator, owned::OwnedFrame, AllocationFlags},
    FRAME_ALLOCATOR,
};
use addr::frame::Frame;

/// A cached page. This structure is used to cache a page of memory and is simply
/// a read/write wrapper around an owned frame with some additional metadata.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Page {
    frame: OwnedFrame,
    dirty: bool,
}

impl Page {
    #[must_use]
    pub fn new() -> Self {
        let frame = unsafe {
            FRAME_ALLOCATOR
                .lock()
                .allocate_frame(AllocationFlags::ZEROED)
                .expect("Failed to allocate memory for a cached page")
        };

        Self { frame, dirty: true }
    }

    /// Writes the given data to the page at the given offset and make the page
    /// as dirty. The caller must ensure that the offset is valid. The memory
    /// that is not written by this function remains unchanged.
    ///
    /// # Panics
    /// Panics if the caller try to access an offset that is out of bounds.
    pub fn write(&mut self, offset: usize, data: &[u8]) {
        assert!(offset + data.len() <= Frame::SIZE);

        self.dirty = true;
        unsafe {
            let dst = self.frame.addr().as_mut_ptr::<u8>().add(offset);
            let src = data.as_ptr();
            let len = data.len();

            core::ptr::copy_nonoverlapping(src, dst, len);
        }
    }

    /// Reads the given data from the page at the given offset. The caller must
    /// ensure that the offset is valid.
    ///
    /// # Panics
    /// Panics if the caller try to access an offset that is out of bounds.
    pub fn read(&self, offset: usize, data: &mut [u8]) {
        assert!(offset + data.len() <= Frame::SIZE);

        unsafe {
            let src = self.frame.addr().as_ptr::<u8>().add(offset);
            let dst = data.as_mut_ptr();
            let len = data.len();

            core::ptr::copy_nonoverlapping(src, dst, len);
        }
    }

    /// Remove the dirty flag from the page.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Returns true if the page is dirty.
    #[must_use]
    pub fn dirty(&self) -> bool {
        self.dirty
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::new()
    }
}
