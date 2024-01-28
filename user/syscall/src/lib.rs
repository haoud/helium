#![no_std]

pub mod clock;
pub mod mmu;
pub mod serial;
pub mod task;
pub mod vfs;
pub mod video;

/// A string that is stored in the userland address space. It is a structure that are created by
/// the rust syscall library and passed to the kernel, so the kernel can then fetch the string from
/// the userland address space.
///
/// We cannot directly pass an `String` to the kernel, because the layout of an `String` is
/// unspecified and may change between different versions of Rust. Therefore, we use this custom
/// structure that has an fixed layout, allowing us to safely read it from the userland address
/// in the kernel.
#[repr(C)]
pub(crate) struct SyscallString {
    pub data: *mut u8,
    pub len: usize,
}

impl From<&str> for SyscallString {
    fn from(value: &str) -> Self {
        Self {
            data: value.as_ptr() as *mut u8,
            len: value.len(),
        }
    }
}

/// A syscall error code. It is returned by the kernel when a syscall fails. The kernel
/// provides different error codes for each syscall, so errno cannot be used as it.
///
/// This structure guarantees that the error code is always a valid error code (between
/// -4095 and -1), but does not guarantee that the error code is valid for the syscall
/// that was called.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Errno(isize);

impl Errno {
    /// Check if the given error code is a valid error code.
    #[must_use]
    pub fn valid(code: isize) -> bool {
        (-4095..0).contains(&code)
    }

    /// Get the error code as an isize.
    #[must_use]
    pub fn code(&self) -> isize {
        self.0
    }
}

// A struct that contains all the syscall numbers used by the kernel.
#[non_exhaustive]
#[repr(u64)]
pub enum Syscall {
    TaskExit = 0,
    TaskId = 1,
    TaskSleep = 2,
    TaskYield = 3,
    TaskSpawn = 4,
    SerialRead = 5,
    SerialWrite = 6,
    MmuMap = 7,
    MmuUnmap = 8,
    VideoFramebufferInfo = 9,
    VfsOpen = 10,
    VfsClose = 11,
    VfsRead = 12,
    VfsWrite = 13,
    VfsSeek = 14,
    VfsGetCwd = 15,
    VfsChangeCwd = 16,
    ClockGetTime = 17,
    VfsMkdir = 18,
    VfsRmdir = 19,
    VfsUnlink = 20,
    VfsTruncate = 21,
    VfsStat = 22,
    VfsReaddir = 23,
}

pub fn syscall_return(code: usize) -> Result<usize, Errno> {
    if Errno::valid(code as isize) {
        Err(Errno(-(code as isize)))
    } else {
        Ok(code)
    }
}
