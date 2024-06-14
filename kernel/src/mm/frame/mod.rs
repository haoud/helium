use addr::frame::{Frame, FrameCount};
use bitflags::bitflags;
use lib::byte::ByteSize;

pub mod allocator;
pub mod owned;
pub mod state;

/// A struct used to keep track of some statistics about the physical memory.
#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct Stats {
    /// Total number of frames
    pub total: FrameCount,

    /// Total number of usable frames for allocation
    pub usable: FrameCount,

    /// Total number of allocated frames
    pub allocated: FrameCount,

    /// Total number of reserved frames
    pub reserved: FrameCount,

    /// Total number of kernel frames
    pub kernel: FrameCount,

    /// Total number of poisoned frames
    pub poisoned: FrameCount,
}

impl Stats {
    #[must_use]
    pub const fn uninitialized() -> Self {
        Self {
            total: FrameCount::new(0),
            usable: FrameCount::new(0),
            allocated: FrameCount::new(0),
            reserved: FrameCount::new(0),
            kernel: FrameCount::new(0),
            poisoned: FrameCount::new(0),
        }
    }

    /// Called when frames are deallocated and update the statistics
    /// accordingly to the number of frames deallocated and their flags.
    /// The only flags used are [`FrameFlags::KERNEL`] to track the number
    /// of kernel frames used. Other flags are ignored.
    pub fn frames_deallocated(&mut self, count: usize, flags: FrameFlags) {
        self.allocated.0 -= count;
        if flags.contains(FrameFlags::KERNEL) {
            self.kernel.0 -= count;
        }
    }

    /// Called when frames are allocated and update the statistics accordingly
    /// to the number of frames allocated and their flags. The only flags used
    /// are [`FrameFlags::KERNEL`] to track the number of kernel frames used.
    /// Other flags are ignored.
    pub fn frames_allocated(&mut self, count: usize, flags: FrameFlags) {
        self.allocated.0 += count;
        if flags.contains(FrameFlags::KERNEL) {
            self.kernel.0 += count;
        }
    }

    /// This is a shortcut for [`Stats::frames_deallocated`] with a count of 1.
    pub fn frame_deallocated(&mut self, flags: FrameFlags) {
        self.frames_deallocated(1, flags);
    }

    /// This is a shortcut for [`Stats::frames_allocated`] with a count of 1.
    pub fn frame_allocated(&mut self, flags: FrameFlags) {
        self.frames_allocated(1, flags);
    }
}

impl core::fmt::Debug for Stats {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let allocated = ByteSize(self.allocated.0 * Frame::SIZE);
        let poisoned = ByteSize(self.poisoned.0 * Frame::SIZE);
        let reserved = ByteSize(self.reserved.0 * Frame::SIZE);
        let usable = ByteSize(self.usable.0 * Frame::SIZE);
        let kernel = ByteSize(self.kernel.0 * Frame::SIZE);
        let total = ByteSize(self.total.0 * Frame::SIZE);

        writeln!(f, "Physical memory usage statistics:")?;
        writeln!(f," - Total memory : {} frames ({})", self.total, total)?;
        writeln!(f," - Usable memory : {} frames ({})", self.usable, usable)?;
        writeln!(f," - Poisoned memory : {} frames ({})", self.poisoned, poisoned)?;
        writeln!(f," - Reserved memory : {} frames ({})",self.reserved, reserved)?;
        writeln!(f," - Allocated memory : {} frames ({})", self.allocated, allocated)?;
        writeln!(f," - Kernel memory : {} frames ({})", self.kernel, kernel)?;
        Ok(())
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FrameFlags : u8 {
        /// If set, the frame is poisoned. This means that the frame is not
        /// usable for allocation, either because it does not exist or because
        /// it is poisoned by the firmware (e.g. bad memory)
        const POISONED = 1 << 0;

        /// If set, the frame is reserved. This means that the frame is not
        /// usable for allocation, but can still be used for other purposes
        /// (e.g. framebuffer, memory mapped IO, etc.)
        const RESERVED = 1 << 1;

        /// If set, the frame is free. This means that the frame is usable for
        /// allocation. This flags cannot coexist with [`FrameFlags::RESERVED`]
        /// or [`FrameFlags::POISONED`].
        const FREE = 1 << 2;

        /// If set, that means that the frame has been zeroed. This flags is
        /// only used for free frames to speed up the allocation process.
        const ZEROED = 1 << 3;

        /// If set, the frame is used by the kernel. This is only used to track
        /// the kernel memory usage.
        const KERNEL = 1 << 4;

        /// If set, the frame is a bootloader reclaimable frame. Currently,
        /// this flags does nothing, but it will be used in the future to allow
        /// the kernel to reclaim frames that were allocated by the bootloader
        /// and only used during the boot process.
        const BOOT = 1 << 5;
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct AllocationFlags : u8 {
        /// If set, the allocated frame will be zeroed before being returned
        const ZEROED =  FrameFlags::ZEROED.bits();

        /// If set, the allocated frame will be marked as used by the kernel.
        /// This is only used to track the kernel memory usage.
        const KERNEL = FrameFlags::KERNEL.bits();
    }
}
