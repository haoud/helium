use crate::phys::Physical;
use core::iter::Step;

/// Represents the identifier of a physical memory frame. This is a simple wrapper 
/// around a usize that guarantees that the usize is a valid frame index (meaning
/// that the usize is less than [`Index::MAX`], but it does not guarantee that the
/// frame really exists).
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Index(pub usize);

impl Index {
    const MAX: usize = Physical::MAX / Frame::SIZE;

    /// Creates a new frame index with the given index.
    ///
    /// # Panics
    /// Panics if the index is greater than [`FrameIndex::MAX`] (meaning that the index
    /// does not represent a valid frame).
    #[must_use]
    pub const fn new(index: usize) -> Self {
        assert!(index < Index::MAX);
        Self(index)
    }

    /// Return the first address of the frame. This is the address of the first byte of
    /// the frame, guaranteed to be page aligned.
    #[must_use]
    pub fn address(self) -> Physical {
        Physical::from(self.0 * Frame::SIZE)
    }

    /// Creates a new frame index from the given address.
    ///
    /// # Panics
    /// Panics if the address is not a valid physical address (see [`Physical::new`] for
    /// more information)
    #[must_use]
    pub const fn from_address(addr: usize) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        Self::new(Physical::new(addr).frame_index())
    }
}

impl From<Frame> for Index {
    #[allow(clippy::cast_possible_truncation)]
    fn from(frame: Frame) -> Self {
        Self::new(frame.addr().frame_index())
    }
}

impl From<Physical> for Index {
    #[allow(clippy::cast_possible_truncation)]
    fn from(physical: Physical) -> Self {
        Self::new(physical.frame_index())
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
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Debug for FrameCount {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} frames", self.0)
    }
}

/// Represents a physical memory frame. A Frame is a 4 KiB block of memory, and is 
/// the smallest unit of physical memory that can be allocated. This struct is a 
/// wrapper around a physical address, and guarantees that the address is always
/// page aligned (i.e 4 KiB aligned).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Frame(Physical);

impl Frame {
    /// The size of a frame, in bytes.
    pub const SIZE: usize = 4096;

    /// Creates a new frame from the given physical address. The address must be page aligned
    /// and must be a valid physical address (i.e. it must be less than 2^52).
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
    /// For example, if the address is `0xFFFF_FFF8_0000_1234`, the returned frame will have the
    /// address `0xFFFF_FFF8_0000_1000`.
    #[must_use]
    pub fn truncate<T: Into<Physical>>(address: T) -> Self {
        Self(address.into().page_align_down())
    }

    /// Creates a new frame and rounds the address up to the next page boundary if necessary.
    /// For example, if the address is `0xFFFF_FFF8_0000_1234`, the returned frame will have the
    /// address `0xFFFF_FFF8_0000_2000`.
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
    #[must_use]
    pub fn index(&self) -> Index {
        Index::from(*self)
    }
}

impl From<u64> for Frame {
    /// Creates a new frame from a u64 address.
    ///
    /// # Panics
    /// Panics if the address is not page aligned (4 KiB aligned), or if the address is not a
    /// valid physical address (i.e. it is greater than 2^52)
    fn from(address: u64) -> Self {
        Self::new(Physical::from(address))
    }
}

impl From<usize> for Frame {
    /// Creates a new frame from a usize address.
    ///
    /// # Panics
    /// Panics if the address is not page aligned (4 KiB aligned), or if the address is not a
    /// valid physical address (i.e. it is greater than 2^52)
    fn from(address: usize) -> Self {
        Self::new(Physical::new(address))
    }
}

impl From<Index> for Frame {
    /// Creates a new frame from a frame index.
    ///
    /// # Panics
    /// Panics if the index is greater than [`FrameIndex::MAX`] (meaning that the index does not
    /// represent a valid frame).
    fn from(idx: Index) -> Self {
        Self::new(idx.address())
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
