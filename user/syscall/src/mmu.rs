use super::{syscall_return, Errno, Syscall};

pub const PROT_READ: usize = 1 << 0;
pub const PROT_WRITE: usize = 1 << 1;
pub const PROT_EXEC: usize = 1 << 2;

pub const MAP_PRIVATE: usize = 0;
pub const MAP_FIXED: usize = 1 << 0;
pub const MAP_SHARED: usize = 1 << 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum MapError {
    NoSuchSyscall = 1,
    InvalidAddress,
    InvalidFlags,
    InvalidRange,
    WouldOverlap,
    OutOfMemory,
    UnknownError,
}

impl From<Errno> for MapError {
    fn from(error: Errno) -> Self {
        if error.code() > -(Self::UnknownError as isize) {
            unsafe { core::mem::transmute(error) }
        } else {
            Self::UnknownError
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum UnmapError {
    NoSuchSyscall = 1,
    InvalidRange,
    UnknownError,
}

impl From<Errno> for UnmapError {
    fn from(error: Errno) -> Self {
        if error.code() > -(Self::UnknownError as isize) {
            unsafe { core::mem::transmute(error) }
        } else {
            Self::UnknownError
        }
    }
}

/// Map a region of memory with the given access and flags.
///
/// # Errors
///  Possible errors are:
///  - `Errno::BadAddress`: The given address is not a valid address.
///  - `Errno::InvalidArgument`: It can be either:
///     - The given length is 0
///     - The resulting range is not in user space
///     - An invalid access or flag is given
///     - An invalid combination of flags is given
///  - `Errno::OutOfMemory`: The kernel ran out of memory while trying to map
///     the region.
///  - `Errno::AlreadyExists`: The range already contains a mapping and the
///     `FIXED` flag was set.
///
/// # Safety
/// This function is unsafe because the design of memory mapped data is totally
/// against Rust memory safety. The caller must ensure that it will not break
/// Rust memory safety by mapping a region of memory. It gets even worse if the
/// caller maps a shared region of a file, as the file may be modified by
/// another process at any time. You shoud be VERY careful when using this
/// function. Maybe there is a better way to do what you want to do ?
pub unsafe fn map(base: usize, len: usize, access: usize, flags: usize) -> Result<usize, MapError> {
    let ret: usize;
    core::arch::asm!(
        "syscall",
        in("rax") Syscall::MmuMap as u64,
        in("rsi") base,
        in("rdx") len,
        in("r10") access,
        in("r8") flags,
        lateout("rax") ret,
    );
    match syscall_return(ret) {
        Err(errno) => unsafe { Err(core::mem::transmute(errno)) },
        Ok(ret) => Ok(ret),
    }
}

/// Unmap a region of memory. If the region is not mapped, this function will
/// do nothing. If multiple mappings exist for the same region, all parts
/// included in the given range will be unmapped.
///
/// # Errors
///  Possible errors are:
///  - `Errno::BadAddress`: The given address is not a valid address.
///  - `Errno::InvalidArgument`: The given length is 0 or the resulting range
///  is not in user space.
///
/// # Safety
/// This function is unsafe because the design of memory mapped data is totally
/// against Rust memory safety. The caller must ensure that no reference to the
/// unmapped region is kept after this function returns. Failure to do so will
/// result in undefined behavior.
pub unsafe fn unmap(base: usize, len: usize) -> Result<(), UnmapError> {
    let ret: usize;
    core::arch::asm!(
        "syscall",
        in("rax") Syscall::MmuUnmap as u64,
        in("rsi") base,
        in("rdx") len,
        lateout("rax") ret,
    );

    match syscall_return(ret) {
        Err(errno) => Err(core::mem::transmute(errno)),
        Ok(_) => Ok(()),
    }
}
