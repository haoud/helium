use crate::user;
use crate::user::scheduler::{Scheduler, SCHEDULER};
use crate::user::vmm::area::{self, Area, Type};
use addr::user::{InvalidUserVirtual, UserVirtual};

/// Map a range of virtual addresses.
///
/// # Errors
/// On success, the syscall returns the start address of the mapped area. If the
/// syscall fails, it can return the following errors:
/// - `InvalidArgument`: the `addr` is not page-aligned, the length is zero, the range
///                      is outside of the user virtual address space or if `access`
///                      or `flags` contain unsupported/invalid bits or invalid
///                      combinations of bits
///
/// - `AlreadyExists`: the range overlaps with an existing area and the `FIXED`
///                    flag was set
/// - `OutOfMemory`: the task has no more virtual or physical memory available
///
/// # Panics
/// This function may panic if the current task does not have a VMM (probably
/// a kernel task that tried to make a syscall).
pub fn map(addr: usize, len: usize, access: usize, flags: usize) -> Result<usize, MmapError> {
    let access = area::Access::from_bits(access as u64).ok_or(MmapError::InvalidFlags)?;
    let flags = area::Flags::from_bits(flags as u64).ok_or(MmapError::InvalidFlags)?;
    let end = UserVirtual::try_new(addr + len)?;
    let start = UserVirtual::try_new(addr)?;

    let area = Area::builder()
        .kind(Type::Anonymous)
        .range(start..end)
        .access(access)
        .flags(flags)
        .offset(0)
        .build();

    let range = SCHEDULER
        .current_task()
        .thread()
        .lock()
        .vmm()
        .unwrap()
        .lock()
        .mmap(area)?;

    Ok(range.start.as_usize())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum MmapError {
    NoSuchSyscall = 1,
    InvalidAddress,
    InvalidFlags,
    InvalidRange,
    WouldOverlap,
    OutOfMemory,
    UnknownError,
}

impl From<InvalidUserVirtual> for MmapError {
    fn from(_: InvalidUserVirtual) -> Self {
        Self::InvalidAddress
    }
}

impl From<user::vmm::MmapError> for MmapError {
    fn from(e: user::vmm::MmapError) -> Self {
        match e {
            user::vmm::MmapError::InvalidFlags => Self::InvalidFlags,
            user::vmm::MmapError::InvalidRange => Self::InvalidRange,
            user::vmm::MmapError::WouldOverlap => Self::WouldOverlap,
            user::vmm::MmapError::OutOfVirtualMemory => Self::OutOfMemory,
        }
    }
}

impl From<MmapError> for isize {
    fn from(error: MmapError) -> Self {
        -(error as isize)
    }
}

/// Unmap a range of virtual addresses.
///
/// # Errors
/// On success, the syscall returns 0. If the syscall fails, it can return the
/// following errors:
/// - `InvalidArgument`: the `addr` is not page-aligned, the length is zero or if
///                      the range is outside of the user virtual address space
///
/// # Panics
/// This function may panic if the current task does not have a VMM (probably
/// a kernel task that tried to make a syscall).
pub fn unmap(base: usize, len: usize) -> Result<usize, UnmapError> {
    let end = UserVirtual::try_new(base + len)?;
    let start = UserVirtual::try_new(base)?;

    SCHEDULER
        .current_task()
        .thread()
        .lock()
        .vmm()
        .unwrap()
        .lock()
        .munmap(start..end)?;

    Ok(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum UnmapError {
    NoSuchSyscall = 1,
    InvalidRange,
    UnknownError,
}

impl From<UnmapError> for isize {
    fn from(error: UnmapError) -> Self {
        -(error as isize)
    }
}

impl From<InvalidUserVirtual> for UnmapError {
    fn from(_: InvalidUserVirtual) -> Self {
        Self::InvalidRange
    }
}

impl From<user::vmm::UnmapError> for UnmapError {
    fn from(e: user::vmm::UnmapError) -> Self {
        match e {
            user::vmm::UnmapError::InvalidRange => Self::InvalidRange,
        }
    }
}
