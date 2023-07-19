use super::{SyscallError, SyscallValue};
use crate::{
    mm::vmm::{
        area::{self, Area, Type},
        MmapError,
    },
    user::scheduler,
};
use addr::user::UserVirtual;

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
pub fn map(
    addr: usize,
    len: usize,
    access: usize,
    flags: usize,
) -> Result<SyscallValue, SyscallError> {
    let access = area::Access::from_bits(access as u64).ok_or(SyscallError::InvalidArgument)?;
    let flags = area::Flags::from_bits(flags as u64).ok_or(SyscallError::InvalidArgument)?;
    let end = UserVirtual::try_new(addr + len)?;
    let start = UserVirtual::try_new(addr)?;

    let area = Area::builder()
        .kind(Type::Anonymous)
        .range(start..end)
        .access(access)
        .flags(flags)
        .build();

    let range = scheduler::current_task()
        .thread()
        .lock()
        .vmm()
        .lock()
        .mmap(area)?;

    Ok(usize::from(range.start))
}

impl From<MmapError> for SyscallError {
    fn from(error: MmapError) -> Self {
        match error {
            MmapError::WouldOverlap => Self::AlreadyExists,
            MmapError::OutOfVirtualMemory => Self::OutOfMemory,
            MmapError::InvalidRange | MmapError::InvalidFlags => Self::InvalidArgument,
        }
    }
}
