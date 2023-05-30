#![no_std]
#![feature(step_trait)]

use core::{
    fmt,
    iter::Step,
    ops::{Add, AddAssign, Sub, SubAssign},
};

/// A canonical 64-bit virtual memory address.
///
/// On `x86_64`, only the 48 lower bits of a virtual address can be used. This type guarantees that
/// the address is always canonical, i.e. that the top 17 bits are either all 0 or all 1.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Virtual(u64);

/// An invalid virtual address.
///
/// This type is used to represent an invalid virtual address. It is returned by [`Virtual::try_new`]
/// when the given address is not canonical (see [`Virtual`] for more information).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct InvalidVirtual(u64);

impl Virtual {
    /// Creates a new canonical virtual address.
    ///
    /// # Panics
    /// This function panics if the given address is not canonical.
    #[must_use]
    pub const fn new(address: u64) -> Self {
        match Self::try_new(address) {
            Ok(addr) => addr,
            Err(InvalidVirtual(_)) => panic!("Invalid virtual address: non canonical"),
        }
    }

    /// Tries to create a new canonical virtual address.
    ///
    /// # Errors
    /// This function returns an [`InvalidVirtual`] error if the given address is not canonical, or
    /// a sign extension is performed if 48th bit is set and all bits from 49 to 63 are set to 0.
    pub const fn try_new(address: u64) -> Result<Self, InvalidVirtual> {
        match (address & 0xFFFF_8000_0000_0000) >> 47 {
            0 | 0x1FFFF => Ok(Self(address)),
            1 => Ok(Self::new_truncate(address)),
            _ => Err(InvalidVirtual(address)),
        }
    }

    /// Creates a new canonical virtual address, truncating the address if necessary.
    /// A sign extension is performed if 48th bit is set and all bits from 49 to 63 are set to 0,
    /// and set those bits to 1 in order to make the address canonical.
    #[must_use]
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
    pub const fn new_truncate(addr: u64) -> Self {
        // Some magic with sign extension on signed 64-bit integer
        // It set the sign bit to the 48th bit, and then shift to the right by 16 bits: all bits
        // from 48 to 63 are set to the sign bit
        Self(((addr << 16) as i64 >> 16) as u64)
    }

    /// Creates a new canonical virtual address without checking if it is canonical.
    ///
    /// # Safety
    /// This function is unsafe because it does not check if the given address is canonical. If the
    /// address is not canonical, the behavior is undefined.
    #[must_use]
    pub const unsafe fn new_unchecked(address: u64) -> Self {
        Self(address)
    }

    /// Checks if the given address is canonical.
    #[must_use]
    pub const fn is_canonical(address: u64) -> bool {
        matches!((address & 0xFFFF_8000_0000_0000) >> 47, 0 | 0x1FFFF)
    }

    /// Creates a new canonical virtual address from a pointer. This is a convenience function that
    /// simply casts the pointer address to a `u64`, and then calls [`Self::new`].
    #[must_use]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::new(ptr as u64)
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

    #[must_use]
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Align the address up to the given alignment. If the address is already aligned, this function
    /// does nothing.
    ///
    /// # Panics
    /// This function panics if the given alignment is not a power of two.
    #[must_use]
    pub fn align_up<T>(&self, alignment: T) -> Self
    where
        T: Into<u64>,
    {
        let align: u64 = alignment.into();
        assert!(align.is_power_of_two());
        Self::new_truncate(
            (self.0.checked_add(align - 1)).expect("Overflow during aligning up a virtual address")
                & !(align - 1),
        )
    }

    /// Align the address down to the given alignment. If the address is already aligned, this
    /// function does nothing.
    ///
    /// # Panics
    /// This function panics if the given alignment is not a power of two.
    #[must_use]
    pub fn align_down<T>(&self, alignment: T) -> Self
    where
        T: Into<u64>,
    {
        let align: u64 = alignment.into();
        assert!(align.is_power_of_two());
        Self::new_truncate(self.0 & !(align - 1))
    }

    /// Checks if the address is aligned to the given alignment.
    ///
    /// # Panics
    /// This function panics if the given alignment is not a power of two.
    #[must_use]
    pub fn is_aligned<T>(&self, alignment: T) -> bool
    where
        T: Into<u64>,
    {
        let align: u64 = alignment.into();
        assert!(align.is_power_of_two());
        self.0 & (align - 1) == 0
    }

    /// Align the address up to a page boundary (4 KiB). If the address is already aligned, this
    /// function does nothing.
    #[must_use]
    pub const fn page_align_up(&self) -> Self {
        Self::new_truncate(match self.0.checked_add(0xFFF) {
            Some(addr) => addr & !0xFFF,
            None => panic!("Overflow during aligning up a virtual address"),
        })
    }

    /// Align the address down to a page boundary (4 KiB). If the address is already aligned, this
    /// function does nothing.
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
    pub const fn page_offset(&self) -> u64 {
        self.0 & 0xFFF
    }

    #[must_use]
    pub const fn page_index(self, level: usize) -> usize {
        assert!(level >= 1 && level <= 5);
        self.0 as usize >> 12 >> ((level - 1) * 9) & 0x1FF
    }

    #[must_use]
    pub const fn pt_index(&self) -> usize {
        self.page_index(1)
    }

    #[must_use]
    pub const fn pd_index(&self) -> usize {
        self.page_index(2)
    }

    #[must_use]
    pub const fn pdpt_index(&self) -> usize {
        self.page_index(3)
    }

    #[must_use]
    pub const fn pml4_index(&self) -> usize {
        self.page_index(4)
    }

    #[must_use]
    pub const fn pml5_index(&self) -> usize {
        self.page_index(5)
    }

    /// Checks if the address is in the kernel address space.
    #[must_use]
    pub const fn is_kernel(&self) -> bool {
        self.0 >= 0xFFFF_8000_0000_0000
    }

    /// Checks if the address is in the user address space.
    #[must_use]
    pub const fn is_user(&self) -> bool {
        !self.is_kernel()
    }
}

impl Step for Virtual {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        let steps = end.0.checked_sub(start.0)?;
        if !Virtual::is_canonical(start.0) || !Virtual::is_canonical(end.0) {
            panic!("Steps between non-canonical addresses");
        }
        usize::try_from(steps).ok()
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let new = start.0.checked_add(count as u64)?;
        if !Virtual::is_canonical(new) {
            return None;
        }
        Some(Self::new(new))
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let new = start.0.checked_sub(count as u64)?;
        if !Virtual::is_canonical(new) {
            return None;
        }
        Some(Self::new(new))
    }
}

impl fmt::Binary for Virtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Binary::fmt(&self.0, f)
    }
}

impl fmt::Octal for Virtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Octal::fmt(&self.0, f)
    }
}

impl fmt::LowerHex for Virtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for Virtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl fmt::Pointer for Virtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(self.0 as *const ()), f)
    }
}

impl fmt::Display for Virtual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:016x}", self.0)
    }
}

impl From<Virtual> for u64 {
    fn from(address: Virtual) -> Self {
        address.0
    }
}

impl From<Virtual> for usize {
    fn from(address: Virtual) -> Self {
        address.0 as usize
    }
}

impl From<u64> for Virtual {
    fn from(address: u64) -> Self {
        Self::new(address)
    }
}

impl From<usize> for Virtual {
    fn from(address: usize) -> Self {
        Self::new(address as u64)
    }
}

impl From<Physical> for Virtual {
    fn from(address: Physical) -> Self {
        // The kernel map all the physical memory at 0xFFFF_8000_0000_0000. To convert a physical
        // address to a virtual address, we just need to add 0xFFFF_8000_0000_0000 to the physical
        // address, and then we can access the physical memory from the returned virtual address.
        Self::new(0xFFFF_8000_0000_0000 + address.0)
    }
}

impl Add<Virtual> for Virtual {
    type Output = Virtual;

    fn add(self, rhs: Virtual) -> Self::Output {
        Self::new(self.0 + rhs.0)
    }
}

impl Add<u64> for Virtual {
    type Output = Virtual;

    fn add(self, rhs: u64) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl Add<usize> for Virtual {
    type Output = Virtual;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.0 + rhs as u64)
    }
}

impl AddAssign<Virtual> for Virtual {
    fn add_assign(&mut self, rhs: Virtual) {
        self.0 += rhs.0;
    }
}

impl AddAssign<u64> for Virtual {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl AddAssign<usize> for Virtual {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs as u64;
    }
}

impl Sub<Virtual> for Virtual {
    type Output = Virtual;

    fn sub(self, rhs: Virtual) -> Self::Output {
        Self::new(self.0 - rhs.0)
    }
}

impl Sub<u64> for Virtual {
    type Output = Virtual;

    fn sub(self, rhs: u64) -> Self::Output {
        Self::new(self.0 - rhs)
    }
}

impl Sub<usize> for Virtual {
    type Output = Virtual;

    fn sub(self, rhs: usize) -> Self::Output {
        Self::new(self.0 - rhs as u64)
    }
}

impl SubAssign<Virtual> for Virtual {
    fn sub_assign(&mut self, rhs: Virtual) {
        self.0 -= rhs.0;
    }
}

impl SubAssign<u64> for Virtual {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

impl SubAssign<usize> for Virtual {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs as u64;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Physical(u64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct InvalidPhysical(u64);

impl Physical {
    /// The maximum physical address supported by the architecture.
    pub const MAX: usize = 0x0010_0000_0000_0000;

    /// Creates a new physical address.
    ///
    /// # Panics
    /// If the address is not valid (bits 52-63 must be 0), this function panics.
    #[must_use]
    pub const fn new(address: u64) -> Self {
        match Self::try_new(address) {
            Ok(addr) => addr,
            Err(InvalidPhysical(_)) => panic!("Physical address is not valid (must be 52 bits)"),
        }
    }

    /// Try to create a new physical address.
    ///
    /// # Errors
    /// If the address is not valid (bits 52-63 must be 0), this function returns an error,
    /// containing the invalid address.
    pub const fn try_new(address: u64) -> Result<Self, InvalidPhysical> {
        if address > 0x000F_FFFF_FFFF_FFFF {
            Err(InvalidPhysical(address))
        } else {
            Ok(Self(address))
        }
    }

    /// Creates a new physical address. Bits 52-63 are truncated to 0 if they are set.
    #[must_use]
    pub const fn new_truncate(addr: u64) -> Self {
        // Only keep the lower 52 bits
        Self(addr & 0x000F_FFFF_FFFF_FFFF)
    }

    /// Checks if an address would be valid if it was truncated to 52 bits.
    #[must_use]
    pub const fn is_valid(address: u64) -> bool {
        address <= 0x000F_FFFF_FFFF_FFFF
    }

    /// Creates a new physical address without checking if it is valid.
    ///
    /// # Safety
    /// The address must be valid (bits 52-63 must be 0). If the address is not valid, the behavior
    /// is undefined.
    #[must_use]
    pub const unsafe fn new_unchecked(address: u64) -> Self {
        Self(address)
    }

    /// Creates a new physical address from a pointer. This is a convenience function for
    /// `Physical::new(ptr as u64)`.
    #[must_use]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::new(ptr as u64)
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

    #[must_use]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub fn align_up<T>(&self, alignment: T) -> Self
    where
        T: Into<u64>,
    {
        let align: u64 = alignment.into();
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
        T: Into<u64>,
    {
        let align: u64 = alignment.into();
        assert!(align.is_power_of_two());
        Self::new_truncate(self.0 & !(align - 1))
    }

    #[must_use]
    pub fn is_aligned<T>(&self, alignment: T) -> bool
    where
        T: Into<u64>,
    {
        let align: u64 = alignment.into();
        assert!(align.is_power_of_two());
        self.0 & (align - 1) == 0
    }

    /// Align the address up to a page boundary (4 KiB). If the address is already aligned, this
    /// function does nothing.
    #[must_use]
    pub const fn page_align_up(&self) -> Self {
        Self::new_truncate(match self.0.checked_add(0xFFF) {
            Some(addr) => addr & !0xFFF,
            None => panic!("Overflow during aligning up a physical address"),
        })
    }

    /// Align the address down to a page boundary (4 KiB). If the address is already aligned, this
    /// function does nothing.
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
    pub const fn frame_index(&self) -> u64 {
        self.0 >> 12
    }
}

impl Step for Physical {
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        end.0.checked_sub(start.0).map(|x| x as usize)
    }

    fn forward_checked(start: Self, count: usize) -> Option<Self> {
        let new = start.0.checked_add(count as u64)?;
        if Physical::is_valid(new) {
            Some(Self::new(new))
        } else {
            None
        }
    }

    fn backward_checked(start: Self, count: usize) -> Option<Self> {
        let new = start.0.checked_sub(count as u64)?;
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
        address.0
    }
}

impl From<Physical> for usize {
    fn from(address: Physical) -> Self {
        address.0 as usize
    }
}

impl From<u64> for Physical {
    fn from(address: u64) -> Self {
        Self::new(address)
    }
}

impl From<usize> for Physical {
    fn from(address: usize) -> Self {
        Self::new(address as u64)
    }
}

impl From<Virtual> for Physical {
    fn from(addr: Virtual) -> Self {
        if addr.0 < 0xFFFF_8000_0000_0000 || addr.0 > 0xFFFF_8FFF_FFFF_FFFF {
            panic!("Cannot convert the virtual address {addr} to physical address");
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
        Self::new(self.0 + rhs)
    }
}

impl Add<usize> for Physical {
    type Output = Physical;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.0 + rhs as u64)
    }
}

impl AddAssign<Physical> for Physical {
    fn add_assign(&mut self, rhs: Physical) {
        self.0 += rhs.0;
    }
}

impl AddAssign<u64> for Physical {
    fn add_assign(&mut self, rhs: u64) {
        self.0 += rhs;
    }
}

impl AddAssign<usize> for Physical {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs as u64;
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
        Self::new(self.0 - rhs)
    }
}

impl Sub<usize> for Physical {
    type Output = Physical;

    fn sub(self, rhs: usize) -> Self::Output {
        Self::new(self.0 - rhs as u64)
    }
}

impl SubAssign<Physical> for Physical {
    fn sub_assign(&mut self, rhs: Physical) {
        self.0 -= rhs.0;
    }
}

impl SubAssign<u64> for Physical {
    fn sub_assign(&mut self, rhs: u64) {
        self.0 -= rhs;
    }
}

impl SubAssign<usize> for Physical {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs as u64;
    }
}
