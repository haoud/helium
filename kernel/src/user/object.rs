use super::pointer::Pointer;
use crate::x86_64;
use core::ops::{Deref, DerefMut};

/// An object that is stored in the userland address space. It is a structure that holds an
/// pointer to the object in the userland address space and an copy of the object in the kernel
/// address space. When the `Object` struct is dropped, it will write the object back to the
/// userland address space, so the object in the userland address space will be updated.
///
/// If you only need to read once an object from the userland address space without updating it,
/// then you should use the [`read`] function of this struct instead of creating an `Object`
/// struct, as it will be more efficient when the object will be dropped.
#[derive(Debug)]
pub struct Object<T> {
    /// A pointer to the object in the userland address space.
    ptr: Pointer<T>,

    /// An copy of the object in the kernel address space.
    inner: T,
}

impl<T> Object<T> {
    /// Create an `Object` from the given pointer that resides in the userland memory. This
    /// function will read the object from the userland memory and store it in the `Object`
    /// struct. When the `Object` struct is dropped, it will write the object back to the
    /// userland memory to update the object in the userland memory if any changes were made.
    ///
    /// # Safety
    /// This function is unsafe because it dereference a raw user pointer and use the `copy_from`
    /// function to copy the object from the userland memory. This function is safe only if the
    /// pointer is valid and if the object in userland memory has exactly the same layout as the
    /// object in the kernel: otherwise, this function will cause undefined behavior.
    #[must_use]
    pub unsafe fn new(ptr: Pointer<T>) -> Self {
        Self {
            inner: Self::read(&ptr),
            ptr,
        }
    }

    /// Manually update the object in the userland memory. This function will write the object
    /// back to the userland memory, so the object in the userland memory will be updated. This
    /// function is automatically called when the `Object` struct is dropped, so you do not need
    /// to call this function manually in most cases.
    ///
    /// # Safety
    /// This function is unsafe because it dereference a raw user pointer and use the `copy_from`
    /// function to copy the object from the userland memory. This function is safe if the pointer
    /// is valid and if the object in userland memory has exactly the same layout as the object in
    /// the kernel: otherwise, this function will cause undefined behavior.
    pub unsafe fn update(&mut self) {
        x86_64::user::write(&self.inner, self.ptr.inner());
    }

    /// Read the object from the userland memory and return it. It return a copy of the object
    /// and does not modify the object in the userland memory. This is advantageous to use this
    /// over using the `Object` struct if you does not need to modify the object in the userland
    /// memory.
    ///
    /// # Safety
    /// This function is unsafe because it dereference a raw user pointer and use the `copy_from`
    /// function to copy the object from the userland memory. This function is safe if the pointer
    /// is valid and if the object in userland memory has exactly the same layout as the object in
    /// the kernel: otherwise, this function will cause undefined behavior.
    #[must_use]
    pub unsafe fn read(src: &Pointer<T>) -> T {
        let mut dst = core::mem::MaybeUninit::<T>::uninit();
        x86_64::user::read(src.inner(), dst.as_mut_ptr());
        dst.assume_init()
    }

    /// Write the object to the userland memory. This function will write the object to the
    /// userland memory, so the object in the userland memory will be updated. This function
    /// is advantageous to use this over using the `Object` struct if you does not need to
    /// read the object from the userland memory, but only need to write it.
    ///
    /// # Safety
    /// This function is unsafe because it dereference a raw user pointer and use the `copy_to`
    /// function to copy the object to the userland memory. This function is safe if the pointer
    /// is valid and if the object in userland memory has exactly the same layout as the object in
    /// the kernel: otherwise, this function will cause undefined behavior.
    pub unsafe fn write(dst: &Pointer<T>, src: &T) {
        x86_64::user::write(src, dst.inner());
    }
}

impl<T> Deref for Object<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Object<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> Drop for Object<T> {
    /// Write the object back to the userland memory to update the object in the userland
    /// memory when the `Object` struct is dropped.
    fn drop(&mut self) {
        unsafe {
            self.update();
        }
    }
}
