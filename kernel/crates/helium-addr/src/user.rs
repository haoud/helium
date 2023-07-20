use crate::virt::Virtual;
use core::{fmt, iter::Step};

/// A canonical 64-bit virtual memory address that is guaranteed to be in user space (
/// 0x0000_0000_0000_0000 to 0x0000_7FFF_FFFF_FFFF).
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct UserVirtual(pub(crate) usize);

/// An invalid virtual address.
///
/// This type is used to represent an invalid virtual address. It is returned by
/// [`UserVirtual::try_new`] when the given address is not in user space.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct InvalidUserVirtual(pub(crate) usize);

impl UserVirtual {
    /// Creates a new canonical virtual address.
    ///
    /// # Panics
    /// This function panics if the given address is not canonical.
    #[must_use]
    pub const fn new(address: usize) -> Self {
        match Self::try_new(address) {
            Err(InvalidUserVirtual(_)) => panic!("Invalid user virtual address: non user space"),
            Ok(addr) => addr,
        }
    }

    /// Tries to create a new user virtual address.
    ///
    /// # Errors
    /// Returns [`InvalidUserVirtual`] if the given address is not in user space.
    pub const fn try_new(address: usize) -> Result<Self, InvalidUserVirtual> {
        if address > 0x0000_7FFF_FFFF_FFFF {
            Err(InvalidUserVirtual(address))
        } else {
            Ok(Self(address))
        }
    }

    /// Creates a new canonical virtual address without checking if it is canonical.
    ///
    /// # Safety
    /// This function is unsafe because it does not check if the given address is in user space:
    /// the caller must ensure that the address is in user space. Otherwise, the behavior is
    /// undefined.
    #[must_use]
    pub const unsafe fn new_unchecked(address: usize) -> Self {
        Self(address)
    }

    /// Checks if the given address is in user space.
    #[must_use]
    pub const fn is_user(address: usize) -> bool {
        matches!(Self::try_new(address), Ok(_))
    }

    /// Checks if the given pointer is in user space. If check if all the addresses that
    /// contains the object pointed by the pointer are in user space.
    #[must_use]
    pub fn is_user_ptr<T>(ptr: *const T) -> bool {
        let length = core::mem::size_of::<T>();
        let start = ptr as usize;

        // There is no need to check overflow because `T` should never be big
        // enough to overflow an u64
        Self::is_user(start) && Self::is_user(start + length)
    }

    /// Convert this user virtual address to an usize.
    #[must_use]
    pub const fn as_usize(&self) -> usize {
        self.0
    }

    /// Convert this user virtual address to an u64.
    #[must_use]
    pub const fn as_u64(&self) -> u64 {
        self.0 as u64
    }

    /// Creates a new canonical virtual address from a pointer. This is a convenience function that
    /// simply casts the pointer address to a `u64`, and then calls [`Self::new`].
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

    #[must_use]
    pub const fn null() -> Self {
        Self(0)
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    /// Returns the second last valid user virtual address that is page aligned.
    #[must_use]
    pub const fn second_last_page_aligned() -> Self {
        Self(0x0000_7FFF_FFFF_E000)
    }

    /// Returns the last user valid virtual address that is page aligned.
    #[must_use]
    pub const fn last_page_aligned() -> Self {
        Self(0x0000_7FFF_FFFF_F000)
    }

    /// Returns the last user virtual address.
    #[must_use]
    pub const fn last() -> Self {
        Self(0x0000_7FFF_FFFF_FFFF)
    }

    #[must_use]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Align the address up to the given alignment. If the address is already aligned, this function
    /// does nothing.
    ///
    /// # Panics
    /// This function panics if the given alignment is not a power of two or if the resulting
    /// address is not in user space.
    #[must_use]
    pub fn align_up<T>(&self, alignment: T) -> Self
    where
        T: Into<usize>,
    {
        let align: usize = alignment.into();
        assert!(align.is_power_of_two());
        Self::new(
            (self.0.checked_add(align - 1)).expect("Overflow during aligning up a virtual address")
                & !(align - 1),
        )
    }

    /// Align the address down to the given alignment. If the address is already aligned, this
    /// function does nothing.
    ///
    /// # Panics
    /// This function panics if the given alignment is not a power of two
    #[must_use]
    pub fn align_down<T>(&self, alignment: T) -> Self
    where
        T: Into<usize>,
    {
        let align: usize = alignment.into();
        assert!(align.is_power_of_two());
        Self::new(self.0 & !(align - 1))
    }

    /// Checks if the address is aligned to the given alignment.
    ///
    /// # Panics
    /// This function panics if the given alignment is not a power of two.
    #[must_use]
    pub fn is_aligned<T>(&self, alignment: T) -> bool
    where
        T: Into<usize>,
    {
        let align: usize = alignment.into();
        assert!(align.is_power_of_two());
        self.0 & (align - 1) == 0
    }

    /// Align the address up to a page boundary (4 KiB). If the address is already aligned, this
    /// function does nothing.
    ///
    /// # Panics
    /// This function panics if the resulting address is not in user space.
    #[must_use]
    pub const fn page_align_up(&self) -> Self {
        Self::new(match self.0.checked_add(0xFFF) {
            Some(addr) => addr & !0xFFF,
            None => panic!("Overflow during aligning up a virtual address"),
        })
    }

    /// Align the address down to a page boundary (4 KiB). If the address is already aligned, this
    /// function does nothing.
    #[must_use]
    pub const fn page_align_down(&self) -> Self {
        Self::new(self.0 & !0xFFF)
    }

    /// Checks if the address is aligned to a page boundary (4 KiB).
    #[must_use]
    pub const fn is_page_aligned(&self) -> bool {
        self.0.trailing_zeros() >= 12
    }
}

impl Step for UserVirtual {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        let steps = end.0.checked_sub(start.0)?;
        if !UserVirtual::is_user(start.0) || !UserVirtual::is_user(end.0) {
            panic!("Steps between non-canonical addresses");
        }
        usize::try_from(steps).ok()
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let new = start.0.checked_add(count)?;
        if !UserVirtual::is_user(new) {
            return None;
        }
        Some(Self::new(new))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let new = start.0.checked_sub(count)?;
        if !UserVirtual::is_user(new) {
            return None;
        }
        Some(Self::new(new))
    }
}

impl fmt::Binary for UserVirtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Binary::fmt(&self.0, f)
    }
}

impl fmt::Octal for UserVirtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Octal::fmt(&self.0, f)
    }
}

impl fmt::LowerHex for UserVirtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for UserVirtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl fmt::Pointer for UserVirtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(self.0 as *const ()), f)
    }
}

impl fmt::Display for UserVirtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}", self.0)
    }
}

impl From<UserVirtual> for u64 {
    fn from(address: UserVirtual) -> Self {
        address.0 as u64
    }
}

impl From<UserVirtual> for usize {
    fn from(address: UserVirtual) -> Self {
        address.0 as usize
    }
}

impl TryFrom<Virtual> for UserVirtual {
    type Error = InvalidUserVirtual;

    fn try_from(address: Virtual) -> Result<Self, Self::Error> {
        Self::try_new(address.0 as usize)
    }
}

impl TryFrom<u64> for UserVirtual {
    type Error = InvalidUserVirtual;

    fn try_from(address: u64) -> Result<Self, Self::Error> {
        Self::try_new(address as usize)
    }
}

impl TryFrom<usize> for UserVirtual {
    type Error = InvalidUserVirtual;

    fn try_from(address: usize) -> Result<Self, Self::Error> {
        Self::try_new(address)
    }
}
