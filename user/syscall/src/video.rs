use super::{syscall_return, Errno, Syscall};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ReadInfoError {
    /// There is no such syscall.
    NoSuchSyscall = 1,

    /// One or more of the arguments is located at an invalid address.
    BadAddress,

    /// An I/O error occurred while reading the framebuffer information.
    UnknownError,
}

impl From<Errno> for ReadInfoError {
    fn from(error: Errno) -> Self {
        if error.code() > -(Self::UnknownError as isize) {
            unsafe { core::mem::transmute(error) }
        } else {
            Self::UnknownError
        }
    }
}

/// Information about the framebuffer. It is a very simple structure that only
/// describes the height, width and bits per pixel of the framebuffer.
/// Assumptions must be made about the framebuffer format, such as the order of
/// the color channels and the number of bits per channel. This is still
/// sufficient for now
#[repr(C)]
pub struct FramebufferInfo {
    pub height: u64,
    pub width: u64,
    pub bpp: u16,
}

/// Obtain information about the framebuffer.
///
/// This function returns information about the framebuffer, such as its height,
/// width and bits per pixel. The information is returned in a `FramebufferInfo`
///
/// # Errors
/// See `ReadInfoError` for details.
pub fn framebuffer_info() -> Result<FramebufferInfo, ReadInfoError> {
    let mut framebuffer_info = FramebufferInfo {
        height: 0,
        width: 0,
        bpp: 0,
    };
    let ret: usize;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::VideoFramebufferInfo as u64,
            in("rsi") &mut framebuffer_info as *mut FramebufferInfo,
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(ReadInfoError::from(errno)),
        Ok(_) => Ok(framebuffer_info),
    }
}
