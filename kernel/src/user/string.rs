use super::{object::Object, pointer::Pointer};
use crate::{config::MAX_STR, x86_64};
use addr::user::UserVirtual;
use alloc::string::FromUtf8Error;

/// A string that is stored in the userland address space. It is a structure
/// that are created by the rust syscall library and passed to the kernel, so
/// the kernel can then fetch the string from the userland address space.
///
/// We cannot directly pass an `String` to the kernel, because the layout of an
/// `String` is unspecified and may change between different versions of Rust.
/// Therefore, we use this custom structure that has an fixed layout, allowing
/// us to safely read it from the userland address in the kernel.
#[repr(C)]
pub struct SyscallString {
    data: *mut u8,
    len: usize,
}

/// Represent an UTF-8 string that is stored in the userland address space.
/// This structure is similar to the [`SyscallString`] structure, but it more
/// convenient to use in the kernel as it make some guarantees about the
/// pointer that the [`SyscallString`] structure does not make.
#[derive(Debug)]
pub struct String {
    data: Pointer<u8>,
    len: usize,
}

impl String {
    /// Creates a new user string from a string from a syscall . This function
    /// does not copy the string from the userland address space to the kernel
    /// address space, but simply create a new string with an user pointer to
    /// the string in the userland address space and the length of the string.
    ///
    /// If the pointer contained in the syscall string is invalid, this
    /// function will return `None`.
    #[must_use]
    pub fn new(str: &SyscallString) -> Option<Self> {
        let data = Pointer::new(str.data)?;
        let len = str.len;
        Some(Self { data, len })
    }

    /// Creates a new user string from a user pointer to an userpace
    /// [`SyscallString`]. If the pointer contained in the syscall string
    /// is invalid, this function will return `None`.
    #[must_use]
    pub fn from_raw_ptr(ptr: &Pointer<SyscallString>) -> Option<Self> {
        // SAFETY: This is safe because even if we fill the syscall string with
        // invalid data, it will be caught by the `new` or `fetch` functions
        // and will not cause undefined behavior.
        unsafe { Self::new(&Object::read(ptr)) }
    }

    /// Fetch an string from the userland address space. This function will
    /// copy the string from the userland address space to the kernel address
    /// space and return it as an `String`. All modifications to the returned
    /// string will not affect the userland string.
    ///
    /// # Errors
    /// This function will return an error if any of the following conditions
    /// are met (see the [`FetchError`] enum for more details):
    /// - The user pointer is invalid: not mapped, not readable or not in the
    ///     userland address
    /// - The string is longer than [`MAX_STR`] bytes
    /// - The string is not valid utf-8
    ///
    /// # Safety
    /// This function is unsafe because it dereference a raw user pointer and
    /// use the `copy_from` function to copy the object from the userland
    /// memory. This function is safe only if the pointer is valid and if the
    /// object in userland memory has exactly the same layout as the object in
    /// the kernel: otherwise, this function will cause undefined behavior.
    pub fn fetch(&self) -> Result<alloc::string::String, FetchError> {
        // Check if the string is too long to be handled by the kernel.
        if self.len > MAX_STR {
            return Err(FetchError::StringTooLong);
        }

        // Check if the string is entirely in the userland address space.
        if !UserVirtual::is_user(self.data.inner() as usize + self.len) {
            return Err(FetchError::InvalidMemory);
        }

        // Allocate a vector with the same size as the string and prepare the copy
        let mut vector = Vec::with_capacity(self.len);
        let dst = vector.as_mut_ptr();
        let src = self.data.inner();
        let len = self.len;

        // SAFETY: This is safe because we checked that the string is entirely
        // in the userland address space and that the string is not too long to
        // be handled by the kernel. Data race are permitted here because the
        // string resides in the userland address space and the kernel cannot
        // prevent data races in the userland address space: it is the
        // responsability of the user program.
        unsafe {
            x86_64::user::copy_from(src, dst, len);
            vector.set_len(len);
            Ok(alloc::string::String::from_utf8(vector)?)
        }
    }
}

/// An enum that represents an error that can occur when fetching an string
/// from the userland address space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchError {
    /// The pointer is invalid: it may be not mapped, not accessible in read
    /// mode or not in the userland address space.
    InvalidMemory,

    /// The string is longer than [`MAX_STR`] bytes.
    StringTooLong,

    /// The string is not valid an valid utf-8 string.
    StringNotUtf8,
}

impl From<FromUtf8Error> for FetchError {
    fn from(_: FromUtf8Error) -> Self {
        Self::StringNotUtf8
    }
}
