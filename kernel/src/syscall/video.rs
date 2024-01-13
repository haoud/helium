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

/// Write the framebuffer info to an user structure
///
/// # Errors
/// - `SyscallError::BadAddress`: the pointer is not in the user address space.
/// - `0` if the framebuffer info was successfully written to the pointer.
///
/// # Panics
/// Panics if the framebuffer info could not be retrieved from Limine.
pub fn framebuffer_info(info_ptr: usize) -> Result<usize, ReadInfoError> {
    let framebuffer_info_ptr =
        Pointer::<FramebufferInfo>::from_usize(info_ptr).ok_or(ReadInfoError::BadAddress)?;

    let framebuffer = &LIMINE_FRAMEBUFFER
        .get_response()
        .get()
        .expect("Failed to get framebuffer info")
        .framebuffers()[0];

    let framebuffer_info = FramebufferInfo {
        height: framebuffer.height,
        width: framebuffer.width,
        bpp: framebuffer.bpp,
    };

    unsafe {
        Object::write(&framebuffer_info_ptr, &framebuffer_info);
    }
    Ok(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ReadInfoError {
    NoSuchSyscall = 1,
    BadAddress,
    UnknownError,
}

impl From<ReadInfoError> for isize {
    fn from(error: ReadInfoError) -> Self {
        -(error as isize)
    }
}
