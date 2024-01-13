use super::{syscall_return, Syscall, Errno};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ReadInfoError {
    NoSuchSyscall = 1,
    BadAddress,
    UnknownError,
}

impl From<Errno> for ReadInfoError {
    fn from(error: Errno) -> Self {
        if error.code() > Self::UnknownError as isize {
            unsafe { core::mem::transmute(error) }
        } else {
            Self::UnknownError
        }
    }
}

/// Information about the framebuffer. It is a very simple structure that only
/// describes the height, width and bits per pixel of the framebuffer. Assumptions
/// must be made about the framebuffer format, such as the order of the color
/// channels and the number of bits per channel. This is still sufficient for
/// now
#[repr(C)]
pub struct FramebufferInfo {
    pub height: u64,
    pub width: u64,
    pub bpp: u16,
}

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

    // Transmute the return value to ReadInfoError if a valid error code was returned.
    // If the error code is unknown, return an UnknownError.
    match syscall_return(ret) {
        Err(errno) => Err(ReadInfoError::from(errno)),
        Ok(_) => Ok(framebuffer_info),
    }
}
