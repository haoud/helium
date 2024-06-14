use crate::mm::frame::{
    owned::{OwnedFrame, OwnedMemory},
    state::State,
    AllocationFlags, FrameFlags,
};
use addr::{
    frame::{self, Frame},
    virt::Virtual,
};
use core::ops::Range;

/// Additional information about a frame. For this allocator, this structure
/// is empty because the allocator does not need any additional information
/// about a frame.
#[derive(Default, Copy, Clone)]
pub struct FrameInfo;

/// A dummy allocator that allocates frames from the frame state. This
/// allocator is very inefficient and should only be used when no other
/// allocator is available. But it could be easily improved, by saving
/// the last allocated frame index to avoid searching the frame state from
/// the beginning.
///
/// For now, the allocator is used as the global allocator, but it will be
/// replaced by a more efficient allocator in the future, when performance
/// will be more important.
pub struct Allocator {
    pub state: State<FrameInfo>,
}

impl Allocator {
    /// Creates a new allocator from the given memory map. It parse the memory
    /// map and fills the frame array in order to allow the allocation of
    /// physical memory frames, and then initializes the allocator.
    #[must_use]
    pub fn new(mmap: &[&limine::memory_map::Entry]) -> Self {
        Self {
            state: State::new(mmap),
        }
    }
}

unsafe impl super::Allocator for Allocator {
    /// Allocates a frame from the frame state. Returns `None` if no frame is
    /// available, or the owned frame if a frame was successfully allocated.
    /// For more information, see the documentation of the `allocate_range`
    /// method.
    ///
    /// # Safety
    /// This function is unsafe because it is the caller's responsibility to
    /// correctly use the allocated frame. The caller must ensure that the
    /// frame is freed only once, and when the frame is no longer used by any
    /// component.
    unsafe fn allocate_frame(
        &mut self,
        flags: super::AllocationFlags,
    ) -> Option<OwnedFrame> {
        self.allocate_range(1, flags).map(|range| {
            range
                .into_owned_frame()
                .expect("Allocated a single frame but got a range")
        })
    }

    /// Allocates a range of free frames from the frame state. Returns `None`
    /// if no frame is available, or a range of owned frames if a range of
    /// frames was successfully allocated.
    ///
    /// # Warning
    /// Avoid using this method as much as posssibe. It is super, super
    /// inefficient, and should only be used when no allocator is available
    /// and for initialization purposes, when allocation speed is not
    /// important. At this state of kernel development, this is totally
    /// acceptable, but this allocator will be replaced by a more efficient
    /// one in the future.
    ///
    /// # Safety
    /// This function is unsafe because it is the caller's responsibility to
    /// correctly use the allocated frame. The caller must ensure that the
    /// frame is freed only once, and when the frame is no longer used by any
    /// component.
    unsafe fn allocate_range(
        &mut self,
        count: usize,
        flags: AllocationFlags,
    ) -> Option<OwnedMemory> {
        let len = self.state.frames.len();
        let mut i = 0;

        while i + count <= len {
            if self.state.frames[i..i + count]
                .iter()
                .all(|e| e.flags.contains(FrameFlags::FREE))
            {
                let address = Frame::from(frame::Index::new(i)).addr();

                // Mark the frames as allocated and zero them if requested
                for frame in &mut self.state.frames[i..i + count] {
                    if flags.contains(AllocationFlags::KERNEL) {
                        frame.flags.insert(FrameFlags::KERNEL);
                    }
                    if flags.contains(AllocationFlags::ZEROED) {
                        let ptr = Virtual::from(address).as_mut_ptr::<u8>();
                        ptr.write_bytes(0, Frame::SIZE);
                    }
                    frame.flags.remove(FrameFlags::FREE);
                    frame.retain();
                }

                // Update the frame statistics
                let flags = self.state.frames[i].flags;
                self.state.statistics.frames_allocated(count, flags);

                return Some(OwnedMemory::new(Range {
                    start: Frame::from(frame::Index::new(i)),
                    end: Frame::from(frame::Index::new(i + count)),
                }));
            }

            // TODO: Skip all the frames that are not free
            i += 1;
        }
        None
    }

    /// Reference a frame in the frame state, meaning that the frame is used
    /// many times. This method
    /// is unsafe because it can cause memory leaks if the frame is not freed
    /// the same number of times it is referenced
    ///
    /// # Safety
    /// This method is unsafe because it can cause memory leaks if the frame
    /// is not freed the same number of times it is referenced.
    ///
    /// # Panics
    /// This method panics if the frame is not allocated (i.e. if the frame
    /// count is 0)
    unsafe fn reference_frame(&mut self, frame: Frame) {
        let frame = self
            .state
            .frame_info_mut(frame.addr())
            .expect("Invalid frame address");

        assert!(
            !frame.is_free(),
            "Referencing a frame that is not allocated"
        );
        frame.retain();
    }

    /// Decrement the reference count of a frame in the frame state. The frame
    /// is freed only if the frame count is 0, so you should not assume that
    /// the frame is freed after calling this method.
    ///
    /// # Safety
    /// This method is unsafe because it can cause a use-after-free or a double
    /// free if the frame is freed but used after this method is called.
    ///
    /// # Panics
    /// This method panics if the frame is already free.
    unsafe fn deallocate_frame(&mut self, frame: Frame) {
        self.deallocate_range(super::Range {
            start: frame,
            end: frame,
        });
    }

    /// Free a range of frames in the frame state. The frames are freed only if
    /// the frame count is 0, so you should not assume that the frames are
    /// freed after calling this method.
    ///
    /// # Safety
    /// This method is unsafe because it can cause a use-after-free or a double
    /// free if the frame is freed but used after this method is called.
    ///
    /// # Panics
    /// This method panics if one or more frames in the range are already free.
    unsafe fn deallocate_range(&mut self, range: Range<Frame>) {
        let start = frame::Index::from(range.start.addr()).0;
        let end = frame::Index::from(range.end.addr()).0;
        let mut count = 0;

        let flags = self.state.frames[start].flags;
        for frame in &mut self.state.frames[start..end] {
            if frame.release() {
                frame.flags.remove(FrameFlags::ZEROED);
                frame.flags.insert(FrameFlags::FREE);
                if frame.flags.contains(FrameFlags::KERNEL) {
                    frame.flags.remove(FrameFlags::KERNEL);
                }
                count += 1;
            }
        }

        self.state.statistics.frames_deallocated(count, flags);
    }
}
