use super::{SyscallError, SyscallValue};
use crate::{
    limine::LIMINE_FRAMEBUFFER,
    user::{object::Object, pointer::Pointer},
};

#[repr(C)]
pub struct FramebufferInfo {
    pub height: u64,
    pub width: u64,
    pub bpp: u16,
}

pub fn map_framebuffer() {}

/// Write the framebuffer info to an user structure
///
/// # Errors
/// - `SyscallError::BadAddress`: the pointer is not in the user address space.
/// - `0` if the framebuffer info was successfully written to the pointer.
///
/// # Panics
/// Panics if the framebuffer info could not be retrieved from Limine.
pub fn framebuffer_info(info_ptr: usize) -> Result<SyscallValue, SyscallError> {
    let info_ptr = info_ptr as *mut FramebufferInfo;
    let framebuffer_info_ptr = Pointer::try_new(info_ptr).ok_or(SyscallError::BadAddress)?;

    // SAFETY: We checked that the pointer is valid.
    let mut user_framebuffer_info = unsafe { Object::new(framebuffer_info_ptr) };
    let framebuffer = &LIMINE_FRAMEBUFFER
        .get_response()
        .get()
        .expect("Failed to get framebuffer info")
        .framebuffers()[0];

    user_framebuffer_info.height = framebuffer.height;
    user_framebuffer_info.width = framebuffer.width;
    user_framebuffer_info.bpp = framebuffer.bpp;
    Ok(0)
}
