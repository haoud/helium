use super::msr;
use crate::user;
use addr::Virtual;
use core::ops::{Deref, DerefMut};

extern "C" {
    static __percpu_start: [u64; 0];
    static __percpu_end: [u64; 0];
}

/// A guard for a per-cpu variable. This wrapper disables preemption when it is created and
/// enables it when it is dropped. This is absolutely necessary, because otherwise, the CPU
/// could be using the per-cpu variable of another CPU if a context switch happens when the
/// variable is being accessed, which would be a disaster and very hard to debug.
///
/// However, there is no need to disable interruptions: this is the caller responsibility to
/// take care of that if necessary.
pub struct PerCpuGuard<'a, T> {
    inner: &'a T,
}

impl<'a, T> PerCpuGuard<'a, T> {
    pub fn new(inner: &'a T) -> Self {
        Self { inner }
    }
}

impl<'a, T> Deref for PerCpuGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, T> Drop for PerCpuGuard<'a, T> {
    fn drop(&mut self) {
        user::preempt::enable();
    }
}

/// A mutable guard for a per-cpu variable. This structure is similar to `PerCpuGuard`, but it
/// allows to modify the inner value.
pub struct PerCpuGuardMut<'a, T> {
    inner: &'a mut T,
}

impl<'a, T> PerCpuGuardMut<'a, T> {
    pub fn new(inner: &'a mut T) -> Self {
        Self { inner }
    }
}

impl<'a, T> Deref for PerCpuGuardMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, T> DerefMut for PerCpuGuardMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}

impl<'a, T> Drop for PerCpuGuardMut<'a, T> {
    fn drop(&mut self) {
        user::preempt::enable();
    }
}

/// A per-cpu variable. This is a simple wrapper around a value, but it makes sure that every CPU
/// will have its own copy of the variable. As a consequence, this structure is Sync, because it
/// will never be shared between CPUs. However, in order to respect the Rust memory model, it is
/// not possible to modify a per-cpu variable without wrapping it inside a object that allows
/// interior mutability, such as `RefCell` or `Spinlock`.
///
/// # Warning
/// This structure is not intended to be used directly. Instead, it should be used with the
/// `#[per_cpu]` attribute on a static variable, that will wrap the variable inside a `PerCpu`
/// structure and put the variable inside the per-cpu section.
pub struct PerCpu<T> {
    inner: T,
}

impl<T> PerCpu<T> {
    /// Create a new per-cpu variable. This function does not do anything special
    pub const fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Return a guard for the per-cpu variable on the current CPU. This guard will disable
    /// preemption while it is alive, so that the thread will not be switched to another CPU
    /// while it is using a per-cpu variable.
    pub fn local(&self) -> PerCpuGuard<T> {
        unsafe {
            user::preempt::disable();
            PerCpuGuard::new(self.local_unchecked())
        }
    }

    /// Return a mutable guard for the per-cpu variable on the current CPU. This guard will disable
    /// preemption while it is alive, so that the thread will not be switched to another CPU
    /// while it is using a per-cpu variable.
    pub fn local_mut(&mut self) -> PerCpuGuardMut<T> {
        unsafe {
            user::preempt::disable();
            PerCpuGuardMut::new(self.local_mut_unchecked())
        }
    }

    /// Return a reference to the per-cpu variable for the current CPU.
    ///
    /// # Safety
    /// This function is return a reference to the per-cpu variable without any wrapper. This is
    /// unsafe because he caller must ensure that the thread will not be switched to another CPU
    /// while it is using the per-cpu variable. For a safe version of this function, see the
    /// `local` method.
    pub unsafe fn local_unchecked(&self) -> &T {
        let addr = core::ptr::addr_of!(self.inner);
        &*fetch_per_cpu(addr)
    }

    /// Return a mutable reference to the per-cpu variable for the current CPU.
    ///
    /// # Safety
    /// This function is return a reference to the per-cpu variable without any wrapper. This is
    /// unsafe because he caller must ensure that the thread will not be switched to another CPU
    /// while it is using the per-cpu variable. For a safe version of this function, see the
    /// `local_mut` method.
    pub unsafe fn local_mut_unchecked(&mut self) -> &mut T {
        let addr = core::ptr::addr_of!(self.inner);
        &mut *fetch_per_cpu(addr)
    }
}

// SAFETY: This is safe because a per-cpu variable will never be shared between CPUs.
unsafe impl<T> Sync for PerCpu<T> {}

/// Return the per-cpu variable for the current CPU. This function is not intended to be used
/// directly, instead, you should use the `#[per_cpu]` attribute on a static variable.
///
/// # Safety
/// This function is unsafe because it deals with pointer, offsets and MSRs to access the per-cpu
/// variables.
pub unsafe fn fetch_per_cpu<T>(ptr: *const T) -> *mut T {
    debug_assert!(ptr >= core::ptr::addr_of!(__percpu_start).cast::<T>());
    debug_assert!(ptr < core::ptr::addr_of!(__percpu_end).cast::<T>());
    debug_assert!(msr::read(msr::Register::GS_BASE) != 0);

    let per_cpu_start = core::ptr::addr_of!(__percpu_start) as u64;
    let offset = ptr as u64 - per_cpu_start;
    let percpu = msr::read(msr::Register::GS_BASE);
    (percpu + offset) as *mut T
}

/// Set the kernel stack for the current CPU. This will be the stack used when the CPU will enter
/// in the syscall handler.
///
/// # Safety
/// This function is unsafe because the caller must ensure that the stack is valid until another
/// call to this function is made with another stack. The caller must also ensure that the stack
/// is correctly aligned, and big enough to handle the syscall handler.
pub unsafe fn set_kernel_stack(base: Virtual) {
    debug_assert!(base.is_aligned(16u64));
    debug_assert!(base.is_kernel());

    let per_cpu = msr::read(msr::Register::GS_BASE) as *mut u64;
    per_cpu.write(u64::from(base));
}
