use core::ops::{DerefMut, Deref};
use crate::x86_64;

#[derive(Debug)]
pub struct Object<T> {
    ptr: *mut T,
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
    pub unsafe fn new(ptr: *mut T) -> Self {
        Self {
            inner: Self::read(ptr),
            ptr,
        }
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
    pub unsafe fn read(src: *const T) -> T {
        let mut dst = core::mem::MaybeUninit::<T>::uninit();
        x86_64::user::read(src, dst.as_mut_ptr());
        dst.assume_init()
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
    fn drop(&mut self) {
        // Write the object back to the userland memory
        unsafe {
            x86_64::user::write(&self.inner, self.ptr);
        }
    }
}
