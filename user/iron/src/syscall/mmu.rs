use super::{Errno, Syscall};
use bitflags::bitflags;

bitflags! {
    /// Access flags for memory regions.
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Access : u64 {
        /// Read access. On x86_64, this flags is always implied when mapping
        /// a region of memory.
        const READ = 1 << 0;

        /// Write access. Allow writing to the region. This flag also implies
        /// read access, even if the `READ` flag is not set.
        const WRITE = 1 << 1;

        /// Execute access. Allow executing code in the region. This flags also
        /// implies read access, even if the `READ` flag is not set.
        const EXECUTE = 1 << 2;

        /// Read and write access combined together.
        const READ_WRITE = Self::READ.bits() | Self::WRITE.bits();

        /// Read and execute access combined together.
        const READ_EXECUTE = Self::READ.bits() | Self::EXECUTE.bits();

        /// Read, write and execute access combined together.
        const ALL = Self::READ.bits() | Self::WRITE.bits() | Self::EXECUTE.bits();
    }

    /// Flags for memory regions.
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Flags : u64 {
        /// The region is private to the process. Any modification to the region will not
        /// be visible to other processes that have mapped the region. This is the default
        /// behavior if the `SHARED` flag is not set.
        const PRIVATE = 0;

        /// The region must be mapped at the given address. If the region cannot be mapped
        /// at the given address, the mapping will fail.
        const FIXED = 1 << 0;

        /// The region is shared between processes, and any modification to the region
        /// will be visible to all processes that have mapped the region. Currently, this
        /// flag is ignored.
        const SHARED = 1 << 1;

        /// Allow the region to grow up. Only used internally by the kernel, this flags
        /// has no effect for the user.
        const GROW_UP = 1 << 2;

        /// Allow the region to grow down. Only used internally by the kernel, this flags
        /// has no effect for the user.
        const GROW_DOWN = 1 << 3;

        /// Permanent mapping. This flags is reserved for kernel usage. Trying to
        /// map a region with this flag will fail.
        const PERMANENT = 1 << 4;
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
///  - `Errno::OutOfMemory`: The kernel ran out of memory while trying to map the region.
///  - `Errno::AlreadyExists`: The range already contains a mapping and the `FIXED` flag
///    was set.
///
/// # Safety
/// This function is unsafe because the design of memory mapped data is totally against Rust
/// memory safety. The caller must ensure that it will not break Rust memory safety by
/// mapping a region of memory. It gets even worse if the caller maps a shared region of
/// a file, as the file may be modified by another process at any time. You shoud be VERY careful
/// when using this function. Maybe there is a better way to do what you want to do ?
pub unsafe fn map(base: usize, len: usize, access: Access, flags: Flags) -> Result<usize, Errno> {
    let ret: usize;
    core::arch::asm!(
        "syscall",
        in("rax") Syscall::MmuMap as u64,
        in("rsi") base,
        in("rdx") len,
        in("r10") access.bits(),
        in("r8") flags.bits(),
        lateout("rax") ret,
    );

    match Errno::from_syscall_return(ret) {
        Some(errno) => Err(errno),
        None => Ok(ret),
    }
}

/// Unmap a region of memory. If the region is not mapped, this function will do nothing.
/// If multiple mappings exist for the same region, all parts included in the given range
/// will be unmapped.
///
/// # Errors
///  Possible errors are:
///  - `Errno::BadAddress`: The given address is not a valid address.
///  - `Errno::InvalidArgument`: The given length is 0 or the resulting range is not in
///     user space.
///
/// # Safety
/// This function is unsafe because the design of memory mapped data is totally against Rust
/// memory safety. The caller must ensure that no reference to the unmapped region is kept
/// after this function returns. Failure to do so will result in undefined behavior.
pub unsafe fn unmap(base: usize, len: usize) -> Result<(), Errno> {
    unsafe {
        let ret: usize;
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::MmuUnmap as u64,
            in("rsi") base,
            in("rdx") len,
            lateout("rax") ret,
        );

        match Errno::from_syscall_return(ret) {
            Some(errno) => Err(errno),
            None => Ok(()),
        }
    }
}
