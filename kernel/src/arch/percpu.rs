use super::msr;
use addr::Virtual;
use core::ops::Deref;

extern "C" {
    static __percpu_start: [u64; 0];
    static __percpu_end: [u64; 0];
}

/// A guard that can be used to access a per-cpu variable. This simply dereferences the pointer
/// to the per-cpu variable, but it make sure that no context switch can happen while the variable
/// is being accessed: this is a kernel bug and must be fixed.
pub struct PerCpuGuard<'a, T> {
    inner: &'a T,
}

impl<'a, T> Deref for PerCpuGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, T> Drop for PerCpuGuard<'a, T> {
    fn drop(&mut self) {
        // TODO: Enable preemption
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
    /// Create a new per-cpu variable with the given value.
    pub const fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Return the variable for the current CPU.
    ///
    /// # Safety
    /// This function is safe to use as long as the caller is sure that the per-cpu variable is
    /// initialized by the SMP initialization code.
    pub fn local(&self) -> PerCpuGuard<T> {
        // TODO: Disable preemption
        let addr = core::ptr::addr_of!(self.inner);
        let data = unsafe { &*fetch_per_cpu(addr) };
        PerCpuGuard { inner: data }
    }
}

unsafe impl<T> Sync for PerCpu<T> {}

/// Return the per-cpu variable for the current CPU. This function is not intended to be used
/// directly, instead, you should use the `#[per_cpu]` attribute on a static variable.
///
/// # Safety
/// This function is unsafe because it deals with pointer, offsets and MSRs to access the per-cpu
/// variables. This function should note be used directly, but with the `#[per_cpu]` attribute.
/// The returned pointer is a mutable one, and it is the caller's responsibility to ensure that
/// this pointer would be correctly used.
pub unsafe fn fetch_per_cpu<T>(ptr: *const T) -> *mut T {
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
    let per_cpu = msr::read(msr::Register::GS_BASE) as *mut u64;
    per_cpu.write(u64::from(base));
}
