use super::{Errno, Syscall};

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

pub fn framebuffer_info() -> Result<FramebufferInfo, Errno> {
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

    match Errno::from_syscall_return(ret) {
        Some(errno) => Err(errno),
        None => Ok(framebuffer_info),
    }
}
