use super::allocator::Allocator;
use crate::mm::FRAME_ALLOCATOR;
use addr::frame::Frame;
use core::{
    mem::ManuallyDrop,
    ops::{Deref, Range},
};

/// A struct that represents a single Frame that are owned by this struct.
/// When dropped, the frame will be automatically deallocated. This is useful
/// to improve the manual memory management of the kernel, avoid memory leaks
/// when used correctly.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedFrame {
    frame: Frame,
}

impl OwnedFrame {
    /// Creates a new [`OwnedFrame`] from the given frame. When dropped, the
    /// frame will be automatically deallocated.
    ///
    /// # Safety
    /// This function is unsafe because it takes ownership of the given frame.
    /// However, it is the caller's responsibility to ensure that the given
    /// frame are exclusively used by this [`OwnedFrame`] instance after it is
    /// created. If the frame are used outside of this [`OwnedFrame`] instance,
    /// the behavior is undefined.
    #[must_use]
    pub unsafe fn new(frame: Frame) -> Self {
        Self { frame }
    }

    /// Consumes this [`OwnedMemory`] and returns the frame that it owns and
    /// does not deallocate it.
    #[must_use]
    pub fn into_inner(self) -> Frame {
        let mut owned = ManuallyDrop::new(self);
        core::mem::take(&mut owned.frame)
    }
}

impl Deref for OwnedFrame {
    type Target = Frame;
    fn deref(&self) -> &Self::Target {
        &self.frame
    }
}

impl Drop for OwnedFrame {
    /// Deallocates the frame owned by this [`OwnedMemory`] instance. Since
    /// this struct should have an exclusive ownership of the frame, this
    /// method should effectively deallocate the frame in addition to
    /// decreasing the reference count of the frame.
    fn drop(&mut self) {
        unsafe {
            FRAME_ALLOCATOR
                .lock()
                .deallocate_frame(core::mem::take(&mut self.frame));
        }
    }
}

/// A struct that represents a range of frames that are owned by this struct.
/// When dropped, the frames will be automatically deallocated. This is useful
/// to improve the manual memory management of the kernel, avoid memory leaks
/// when used correctly.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct OwnedMemory {
    frames: Range<Frame>,
}

impl OwnedMemory {
    /// Creates a new [`OwnedMemory`] from the given range of frames. When
    /// dropped, the frames will be automatically deallocated.
    ///
    /// # Safety
    /// This function is unsafe because it takes ownership of the given frames.
    /// However, it is the caller's responsibility to ensure that the given
    /// frames are exclusively used by this [`OwnedMemory`] instance after it
    /// is created. If the frames are used outside of this [`OwnedMemory`]
    /// instance, the behavior is undefined.
    #[must_use]
    pub unsafe fn new(frames: Range<Frame>) -> Self {
        Self { frames }
    }

    /// Consumes this [`OwnedMemory`] and return an owned frame instead if the
    /// range of frames that it owns contains only one frame. If the range of
    /// frames contains more than one frame, this method returns `None` and
    /// deallocate the frames.
    #[must_use]
    pub fn into_owned_frame(self) -> Option<OwnedFrame> {
        if self.frames.end.addr() == self.frames.start.end() {
            unsafe {
                Some(OwnedFrame::new(ManuallyDrop::new(self).frames.start))
            }
        } else {
            None
        }
    }

    /// Consumes this [`OwnedMemory`] and returns the range of frames that it
    /// owns without deallocating them.
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
    /// Deallocates the frames owned by this [`OwnedMemory`] instance. Since
    /// this struct should have an exclusive ownership of the frames, this
    /// method should effectively deallocate the frames in addition to
    /// decreasing the reference count of the frames.
    fn drop(&mut self) {
        unsafe {
            FRAME_ALLOCATOR
                .lock()
                .deallocate_range(core::mem::take(&mut self.frames));
        }
    }
}
