use addr::Physical;
use bitflags::bitflags;
use core::iter::Step;
use utils::byte::ByteSize;

pub mod allocator;
pub mod state;

/// Represents the identifier of a physical memory frame. This is a simple wrapper around a usize
/// that guarantees that the usize is a valid frame index (meaning that the usize is less than
/// [`FrameIndex::MAX`], but it does not guarantee that the frame really exists).
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameIndex(pub usize);

impl FrameIndex {
    const MAX: usize = Physical::MAX / Frame::SIZE;

    /// Creates a new frame index with the given index.
    ///
    /// # Panics
    /// Panics if the index is greater than [`FrameIndex::MAX`] (meaning that the index does not
    /// represent a valid frame).
    #[must_use]
    pub const fn new(index: usize) -> Self {
        assert!(index < FrameIndex::MAX);
        Self(index)
    }

    /// Creates a new frame index from the given address.
    ///
    /// # Panics
    /// Panics if the address is not a valid physical address (see [`Physical::new`] for more
    /// information)
    #[must_use]
    pub const fn from_address(addr: u64) -> Self {
        Self::new(Physical::new(addr).frame_index() as usize)
    }
}

impl From<Frame> for FrameIndex {
    fn from(frame: Frame) -> Self {
        Self::new(frame.addr().frame_index() as usize)
    }
}

impl From<Physical> for FrameIndex {
    fn from(physical: Physical) -> Self {
        Self::new(physical.frame_index() as usize)
    }
}

/// A wrapper around a usize that represents a number of frames.
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameCount(pub usize);

impl FrameCount {
    #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }
}

impl core::fmt::Display for FrameCount {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} frames ({})",
            self.0,
            ByteSize::new(self.0 * Frame::SIZE)
        )
    }
}

/// Represents a physical memory frame. A Frame is a 4 KiB block of memory, and is the smallest
/// unit of physical memory that can be allocated.
/// This struct is a wrapper around a physical address and guarantees that the address is page
/// aligned (4 KiB aligned).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Frame(Physical);

impl Frame {
    pub const SIZE: usize = 4096;

    /// Creates a new frame
    ///
    /// # Panics
    /// Panics if the address is not page aligned (4 KiB aligned).
    #[must_use]
    pub fn new<T: Into<Physical>>(address: T) -> Self {
        let address = address.into();
        assert!(address.is_page_aligned());
        Self(address)
    }

    /// Creates a new frame and truncates the address to the previous page boundary if necessary.
    /// For example, if the address is 0xFFFF_FFF8_0000_1234, the returned frame will have the
    /// address 0xFFFF_FFF8_0000_1000.
    #[must_use]
    pub fn truncate<T: Into<Physical>>(address: T) -> Self {
        Self(address.into().page_align_down())
    }

    /// Creates a new frame and rounds the address up to the next page boundary if necessary.
    /// For example, if the address is 0xFFFF_FFF8_0000_1234, the returned frame will have the
    /// address 0xFFFF_FFF8_0000_2000.
    #[must_use]
    pub fn upper<T: Into<Physical>>(address: T) -> Self {
        Self(address.into().page_align_up())
    }

    /// Check if the frame contains the given address.
    #[must_use]
    pub fn contains(&self, address: Physical) -> bool {
        address >= self.0 && address < self.0 + Frame::SIZE
    }

    /// Return the physical address of the frame. This is the address of the first byte of the
    /// frame, guaranteed to be page aligned. This is the same as [`Frame::start`].
    #[must_use]
    pub const fn addr(&self) -> Physical {
        self.0
    }

    /// Return the physical address of the frame. This is the address of the first byte of the
    /// frame, guaranteed to be page aligned. This is the same as [`Frame::addr`].
    #[must_use]
    pub const fn start(&self) -> Physical {
        self.0
    }

    /// Return the physical address of the last byte of the frame. The returned address is not
    /// included in the frame.
    #[must_use]
    pub fn end(&self) -> Physical {
        self.0 + Frame::SIZE
    }

    /// Return the index of the frame. This is an identifier that is unique for each different
    /// frame. The first frame  in memory has index 0, the second frame has index 1, etc.
    pub fn index(&self) -> FrameIndex {
        FrameIndex::from(self.clone())
    }
}

impl From<u64> for Frame {
    /// Creates a new frame from a u64 address.
    ///
    /// # Panics
    /// Panics if the address is not page aligned (4 KiB aligned), or if the address is not a
    /// valid physical address (i.e. it is greater than 2^52)
    fn from(address: u64) -> Self {
        Self::new(Physical::new(address))
    }
}

impl From<usize> for Frame {
    /// Creates a new frame from a usize address.
    ///
    /// # Panics
    /// Panics if the address is not page aligned (4 KiB aligned), or if the address is not a
    /// valid physical address (i.e. it is greater than 2^52)
    fn from(address: usize) -> Self {
        Self::new(Physical::new(address as u64))
    }
}

impl From<FrameIndex> for Frame {
    /// Creates a new frame from a frame index.
    ///
    /// # Panics
    /// Panics if the index is greater than [`FrameIndex::MAX`] (meaning that the index does not
    /// represent a valid frame).
    fn from(idx: FrameIndex) -> Self {
        Self::new(idx.0 as u64 * Frame::SIZE as u64)
    }
}

impl Step for Frame {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        Some(usize::from(end.0 - start.0) / Frame::SIZE)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let offset = count * Frame::SIZE;
        if start.0 + offset < Physical::from(Physical::MAX) {
            return Some(Self::new(start.0 + offset));
        }
        None
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let offset = count * Frame::SIZE;
        if offset <= start.0.into() {
            return Some(Self::new(start.0 - count * Frame::SIZE));
        }
        None
    }
}

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

    /// Called when frames are deallocated and update the statistics accordingly to the number of
    /// frames deallocated and their flags.
    /// The only flags used are [`FrameFlags::KERNEL`] to track the number of kernel frames used.
    /// Other flags are ignored.
    pub fn frames_deallocated(&mut self, count: usize, flags: FrameFlags) {
        self.allocated.0 -= count;
        if flags.contains(FrameFlags::KERNEL) {
            self.kernel.0 -= count;
        }
    }

    /// Called when frames are allocated and update the statistics accordingly to the number of
    /// frames allocated and their flags.
    /// The only flags used are [`FrameFlags::KERNEL`] to track the number of kernel frames used.
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

impl core::fmt::Display for Stats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(f, "Total frames: {}", self.total)?;
        writeln!(f, "Usable frames: {}", self.usable)?;
        writeln!(f, "Allocated frames: {}", self.allocated)?;
        writeln!(f, "Reserved frames: {}", self.reserved)?;
        writeln!(f, "Kernel frames: {}", self.kernel)?;
        writeln!(f, "Poisoned frames: {}", self.poisoned)?;
        Ok(())
    }
}

bitflags! {
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FrameFlags : u8 {
        /// If set, the frame is poisoned. This means that the frame is not usable for allocation,
        /// either because it does not exist or because it is poisoned by the firmware (e.g. bad
        /// memory)
        const POISONED = 1 << 0;

        /// If set, the frame is reserved. This means that the frame is not usable for allocation,
        /// but can still be used for other purposes (e.g. framebuffer, memory mapped IO, etc.)
        const RESERVED = 1 << 1;

        /// If set, the frame is free. This means that the frame is usable for allocation. This
        /// flags cannot coexist with [`FrameFlags::RESERVED`] or [`FrameFlags::POISONED`].
        const FREE = 1 << 2;

        /// If set, that means that the frame has been zeroed. This flags is only used for free
        /// frames to speed up the allocation process.
        const ZEROED = 1 << 3;

        /// If set, the frame is used by the kernel. This is only used to track the kernel memory
        /// usage.
        const KERNEL = 1 << 4;

        /// If set, the frame is a bootloader reclaimable frame. Currently, this flags does
        /// nothing, but it will be used in the future to allow the kernel to reclaim frames
        /// that were allocated by the bootloader and only used during the boot process.
        const BOOT = 1 << 5;
    }

    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct AllocationFlags : u8 {
        /// If set, the allocated frame will be zeroed before being returned
        const ZEROED =  FrameFlags::ZEROED.bits();

        /// If set, the allocated frame will be marked as used by the kernel. This is only
        /// used to track the kernel memory usage.
        const KERNEL = FrameFlags::KERNEL.bits();
    }
}
