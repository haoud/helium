use crate::virt::Virtual;
use core::{
    fmt,
    iter::Step,
    ops::{Add, AddAssign, Sub, SubAssign},
};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Physical(pub(crate) usize);

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct InvalidPhysical(pub(crate) usize);

impl Physical {
    /// The maximum physical address supported by the x86_64 architecture.
    pub const MAX: usize = 0x000F_FFFF_FFFF_FFFF;

    /// Creates a new physical address.
    ///
    /// # Panics
    /// If the address is not valid (bits 52-63 must be 0), this function panics.
    #[must_use]
    pub const fn new(address: usize) -> Self {
        match Self::try_new(address) {
            Err(InvalidPhysical(_)) => {
                panic!("Physical address is not valid (must be 52 bits)")
            }
            Ok(addr) => addr,
        }
    }

    /// Try to create a new physical address.
    ///
    /// # Errors
    /// If the address is not valid (bits 52-63 must be 0), this function
    /// returns an error, containing the invalid address.
    pub const fn try_new(address: usize) -> Result<Self, InvalidPhysical> {
        if address > Self::MAX {
            Err(InvalidPhysical(address))
        } else {
            Ok(Self(address))
        }
    }

    /// Creates a new physical address. Bits 52-63 are truncated to 0 if they
    /// are set.
    #[must_use]
    pub const fn new_truncate(addr: usize) -> Self {
        // Only keep the lower 52 bits
        Self(addr & 0x000F_FFFF_FFFF_FFFF)
    }

    /// Checks if an address would be valid if it was truncated to 52 bits.
    #[must_use]
    pub const fn is_valid(address: usize) -> bool {
        address <= 0x000F_FFFF_FFFF_FFFF
    }

    /// Creates a new physical address without checking if it is valid.
    ///
    /// # Safety
    /// The address must be valid (bits 52-63 must be 0). If the address is
    /// not valid, the behavior is undefined.
    #[must_use]
    pub const unsafe fn new_unchecked(address: usize) -> Self {
        Self(address)
    }

    /// Creates a new physical address from a pointer. This is a convenience
    /// function for `Physical::new(ptr as usize)`.
    #[must_use]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::new(ptr as usize)
    }

    #[must_use]
    pub const fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    #[must_use]
    pub const fn as_mut_ptr<T>(&self) -> *mut T {
        self.as_ptr::<T>() as *mut T
    }

    /// Convert this physical address to an usize.
    #[must_use]
    pub const fn as_usize(&self) -> usize {
        self.0
    }

    /// Convert this physical address to an usize.
    #[must_use]
    pub const fn as_u64(&self) -> u64 {
        self.0 as u64
    }

    #[must_use]
    pub const fn null() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub fn align_up<T>(&self, alignment: T) -> Self
    where
        T: Into<usize>,
    {
        let align: usize = alignment.into();
        assert!(align.is_power_of_two());
        Self::new_truncate(
            (self.0.checked_add(align - 1))
                .expect("Overflow during aligning up a physical address")
                & !(align - 1),
        )
    }

    #[must_use]
    pub fn align_down<T>(&self, alignment: T) -> Self
    where
        T: Into<usize>,
    {
        let align: usize = alignment.into();
        assert!(align.is_power_of_two());
        Self::new_truncate(self.0 & !(align - 1))
    }

    #[must_use]
    pub fn is_aligned<T>(&self, alignment: T) -> bool
    where
        T: Into<usize>,
    {
        let align: usize = alignment.into();
        assert!(align.is_power_of_two());
        self.0 & (align - 1) == 0
    }

    /// Align the address up to a page boundary (4 KiB). If the address is
    /// already aligned, this function does nothing.
    #[must_use]
    pub const fn page_align_up(&self) -> Self {
        Self::new_truncate(match self.0.checked_add(0xFFF) {
            Some(addr) => addr & !0xFFF,
            None => panic!("Overflow during aligning up a physical address"),
        })
    }

    /// Align the address down to a page boundary (4 KiB). If the address is
    /// already aligned, this function does nothing.
    #[must_use]
    pub const fn page_align_down(&self) -> Self {
        Self::new_truncate(self.0 & !0xFFF)
    }

    /// Checks if the address is aligned to a page boundary (4 KiB).
    #[must_use]
    pub const fn is_page_aligned(&self) -> bool {
        self.0.trailing_zeros() >= 12
    }

    #[must_use]
    pub const fn frame_index(&self) -> usize {
        self.0 >> 12
    }
}

impl Step for Physical {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        end.0.checked_sub(start.0)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let new = start.0.checked_add(count)?;
        if Physical::is_valid(new) {
            Some(Self::new(new))
        } else {
            None
        }
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let new = start.0.checked_sub(count)?;
        if Physical::is_valid(new) {
            Some(Self::new(new))
        } else {
            None
        }
    }
}

impl fmt::Binary for Physical {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Binary::fmt(&self.0, f)
    }
}

impl fmt::Octal for Physical {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Octal::fmt(&self.0, f)
    }
}

impl fmt::LowerHex for Physical {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for Physical {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl fmt::Pointer for Physical {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(self.0 as *const ()), f)
    }
}

impl fmt::Display for Physical {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}", self.0)
    }
}

impl From<Physical> for u64 {
    fn from(address: Physical) -> Self {
        address.0 as u64
    }
}

impl From<Physical> for usize {
    fn from(address: Physical) -> Self {
        address.0
    }
}

impl From<u64> for Physical {
    fn from(address: u64) -> Self {
        Self::new(address as usize)
    }
}

impl From<usize> for Physical {
    fn from(address: usize) -> Self {
        Self::new(address)
    }
}

impl From<Virtual> for Physical {
    fn from(addr: Virtual) -> Self {
        if addr.0 < 0xFFFF_8000_0000_0000 || addr.0 > 0xFFFF_8FFF_FFFF_FFFF {
            panic!(
                "Cannot convert the virtual address {addr} to physical address"
            );
        }
        Self::new(addr.0 - 0xFFFF_8000_0000_0000)
    }
}

impl Add<Physical> for Physical {
    type Output = Physical;

    fn add(self, rhs: Physical) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}

impl Add<u64> for Physical {
    type Output = Physical;

    fn add(self, rhs: u64) -> Self::Output {
        Self::new(self.0 + rhs as usize)
    }
}

impl Add<usize> for Physical {
    type Output = Physical;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl AddAssign<Physical> for Physical {
    fn add_assign(&mut self, rhs: Physical) {
        self.0 += rhs.0;
    }
}

impl AddAssign<u64> for Physical {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs as usize;
    }
}

impl AddAssign<usize> for Physical {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub<Physical> for Physical {
    type Output = Physical;

    fn sub(self, rhs: Physical) -> Self::Output {
        Self::new(self.0 - rhs.0)
    }
}

impl Sub<u64> for Physical {
    type Output = Physical;

    fn sub(self, rhs: u64) -> Self::Output {
        Self::new(self.0 - rhs as usize)
    }
}

impl Sub<usize> for Physical {
    type Output = Physical;

    fn sub(self, rhs: usize) -> Self::Output {
        Self::new(self.0 - rhs)
    }
}

impl SubAssign<Physical> for Physical {
    fn sub_assign(&mut self, rhs: Physical) {
        self.0 -= rhs.0;
    }
}

impl SubAssign<u64> for Physical {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs as usize;
    }
}

impl SubAssign<usize> for Physical {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}
