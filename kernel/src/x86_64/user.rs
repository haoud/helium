use alloc::ffi::CString;
use core::cell::Cell;
use macros::per_cpu;

/// The `USER_OPERATION` variable is used to signal if the current CPU is performing a user
/// operation or not. This is useful to not panic when a unrecoverable page fault occurs in
/// kernel space: if an user operation was in progress, then we can try to kill the process
/// because it is likely that the fault was caused by the user process who tried to access
/// invalid memory or gave an invalid pointer to the kernel. If no user operation was in
/// progress, then we can't do anything and we must panic.
#[per_cpu]
static USER_OPERATION: Cell<bool> = Cell::new(false);

/// Executes the given function while signaling that the current CPU is performing a user
/// operation.
fn perform_user_operation<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    USER_OPERATION.local().set(true);
    let ret = f();
    USER_OPERATION.local().set(false);
    ret
}

/// Checks if the current CPU is currently performing a user operation. This function is used
/// when a page fault occurs to know if the fault was caused by a kernel performing a user
/// operation or not. If the fault was caused by a user operation, then we can try to fix it
/// by killing the process. If the fault was caused by the kernel, then we can't do anything
/// and we must panic.
#[must_use]
pub fn in_operation() -> bool {
    USER_OPERATION.local().get()
}

/// Copy `len` bytes from the given source address to the given destination address. This function
/// should only be used to copy data from user space to kernel space. If you want to copy data
/// from kernel space to user space, then you should use [`copy_to`].
///
/// # Safety
/// This function is unsafe because it dereferences a user raw pointer that could possibly be
/// invalid: it is the caller's responsibility to ensure that the pointer is valid and does not
/// overlap with kernel space. However, the caller does not need to ensure that the memory is
/// readable, as this function will handle page faults and kill the process if necessary.
pub unsafe fn copy_from<T>(src: *const T, dst: *mut T, len: usize) {
    perform_user_operation(|| {
        core::ptr::copy_nonoverlapping(src, dst, len);
    });
}

/// Write the given value to the given address. This function is implemented by an simple call
/// to [`copy_from`] with the same source and destination address and a length of 1. This will
/// copy one `T` from the userland memory to the kernel.
/// 
/// # Safety
/// This function is unsafe because it dereferences a user raw pointer that could possibly be
/// invalid: it is the caller's responsibility to ensure that the pointer is valid and does not
/// overlap with kernel space. However, the caller does not need to ensure that the memory is
/// readable, as this function will handle page faults and kill the
pub unsafe fn read<T>(src: *const T, dst: *mut T) {
    copy_from(src, dst, 1);
}

/// Copy `len` bytes from the given source address to the given destination address. This function
/// should only be used to copy data from kernel space to user space. If you want to copy data
/// from user space to kernel space, then you should use [`copy_from`].
///
/// # Safety
/// This function is unsafe because it dereferences a user raw pointer that could possibly be
/// invalid: it is the caller's responsibility to ensure that the pointer is valid and does not
/// overlap with kernel space. However, the caller does not need to ensure that the memory is
/// readable, as this function will handle page faults and kill the process if necessary.
pub unsafe fn copy_to<T>(src: *const T, dst: *mut T, len: usize) {
    perform_user_operation(|| {
        core::ptr::copy_nonoverlapping(src, dst, len);
    });
}

/// Write the given value to the given address. This function is implemented by an simple call
/// to [`copy_to`] with the same source and destination address and a length of 1. This will
/// copy one `T` from the kernel to the userland memory.
/// 
/// # Safety
/// This function is unsafe because it dereferences a user raw pointer that could possibly be
/// invalid: it is the caller's responsibility to ensure that the pointer is valid and does not
/// overlap with kernel space. However, the caller does not need to ensure that the memory is
/// readable, as this function will handle page faults and kill the
pub unsafe fn write<T>(src: *const T, dst: *mut T) {
    copy_to(src, dst, 1);
}

/// Read an user c-string from the given address. It will try to read all the bytes from the
/// given address until it finds a null byte. If an maximum length is given, then it will read
/// at most this number of bytes: if the string is longer than this number, then it will return
/// `None`.
///
/// # Safety
/// This function is unsafe because it dereferences a raw pointer, what's more is an user pointer.
///
/// # Panics
/// This function should not panic. If it does, then it's a bug in this function code.
#[must_use]
pub unsafe fn read_cstr(src: *const u8) -> Option<CString> {
    // Compute the maximal length of the string
    let len = cstr_length(src);
    perform_user_operation(|| {
        let slice = core::slice::from_raw_parts(src, len);
        Some(CString::new(slice).unwrap())
    })
}

/// Compute the length of an user c-string. It will read all the bytes from the given address
/// until it finds a null byte.
///
/// # Safety
/// This function is unsafe because it dereferences a user raw pointer that could possibly be
/// invalid: it is the caller's responsibility to ensure that the pointer is valid and does not
/// overlap with kernel space. However, the caller does not need to ensure that the memory is
/// readable, as this function will handle page faults and kill the process if necessary.
///
/// # Panics
/// This function should not panic. If it does, then it's a bug in this function code.
#[must_use]
#[allow(clippy::maybe_infinite_iter)]
pub unsafe fn cstr_length(src: *const u8) -> usize {
    perform_user_operation(|| (0..).position(|i| src.add(i).read() == 0).unwrap())
}
