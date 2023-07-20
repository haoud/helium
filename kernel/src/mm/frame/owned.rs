use super::allocator::Allocator;
use crate::mm::FRAME_ALLOCATOR;
use addr::frame::Frame;
use core::{
    mem::ManuallyDrop,
    ops::{Deref, Range},
};

/// A struct that represents a range of frames that are owned by this struct.
/// When dropped, the frames will be automatically deallocated. This is useful
/// to improve the manual memory management of the kernel, avoid memory leaks
/// when used correctly.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedMemory {
    frames: Range<Frame>,
}

impl OwnedMemory {
    /// Creates a new [`OwnedMemory`] from the given range of frames. When dropped, the
    /// frames will be automatically deallocated.
    ///
    /// # Safety
    /// This function is unsafe because it takes ownership of the given frames. However,
    /// it is the caller's responsibility to ensure that the given frames are exclusively
    /// used by this [`OwnedMemory`] instance after it is created. If the frames are used
    /// outside of this [`OwnedMemory`] instance, the behavior is undefined.
    #[must_use]
    pub unsafe fn new(frames: Range<Frame>) -> Self {
        Self { frames }
    }

    /// Creates a new [`OwnedMemory`] from the given frame. When dropped, the frame
    /// will be automatically deallocated.
    ///
    /// # Safety
    /// This function is unsafe because it takes ownership of the given frame. However,
    /// it is the caller's responsibility to ensure that the given frame is exclusively
    /// used by this [`OwnedMemory`] instance after it is created. If the frame is used
    /// outside of this [`OwnedMemory`] instance, the behavior is undefined.
    #[must_use]
    pub unsafe fn frame(frame: Frame) -> Self {
        let next = Frame::new(frame.addr() + Frame::SIZE);
        Self::new(frame..next)
    }

    /// Consumes this [`OwnedMemory`] and returns the range of frames that it owns, and
    /// the frames owned by this [`OwnedMemory`] will not be deallocated.
    #[must_use]
    pub fn into_inner(self) -> Range<Frame> {
        let mut owned = ManuallyDrop::new(self);
        core::mem::take(&mut owned.frames)
    }
}

impl Deref for OwnedMemory {
    type Target = Range<Frame>;
    fn deref(&self) -> &Self::Target {
        &self.frames
    }
}

impl Drop for OwnedMemory {
    /// Deallocates the frames owned by this [`OwnedMemory`] instance. Since this 
    /// struct should have an exclusive ownership of the frames, this method should
    /// effectively deallocate the frames in addition to decreasing the reference
    /// count of the frames.
    fn drop(&mut self) {
        unsafe {
            FRAME_ALLOCATOR
                .lock()
                .deallocate_range(core::mem::take(&mut self.frames));
        }
    }
}
