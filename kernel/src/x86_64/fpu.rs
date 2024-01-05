use super::cpu::{Cr0, Cr4, XCr0};

/// The buffer used to store the FPU state. It must be in a 64 bytes aligned
/// memory region, and that's why it must have its own struct instead of
/// simply using a array in the `State` struct.
#[derive(Clone)]
#[repr(align(64))]
struct Buffer([u8; 4096]);

/// The FPU state.
#[derive(Clone)]
pub struct State {
    inner: Box<Buffer>,
}

impl State {
    /// Create a new FPU state with all registers set to zero.
    #[must_use]
    pub fn zeroed() -> Self {
        Self {
            inner: unsafe { Box::new_zeroed().assume_init() },
        }
    }

    /// Return a mutable pointer to the inner buffer.
    #[must_use]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.inner.as_mut().0.as_mut_ptr()
    }

    /// Return a constant pointer to the inner buffer.
    #[must_use]
    pub fn as_ptr(&self) -> *const u8 {
        self.inner.as_ref().0.as_ptr()
    }
}

/// Setup the FPU by enabling the necessary flags in the CR4 and XCR0 registers to support
/// SSE and AVX instructions. For now, this code makes a lot of assumptions about the CPU
/// and the FPU, but it will be improved in the future by detecting the CPU features and
/// enabling the necessary flags or not.
///
/// # Safety
/// This function is unsafe because it directly touches to the control registers of
/// the CPU, which can lead to undefined behavior if not used properly or if an
/// flags or an instruction is not supported by the CPU.
///
/// # Panics
/// Panics if the CPU does not support SSE or XSAVE instructions.
#[init]
pub unsafe fn setup() {
    assert!(
        (core::arch::x86_64::__cpuid(1).edx & (1 << 25)) != 0,
        "SSE is not supported by the CPU"
    );

    assert!(
        (core::arch::x86_64::__cpuid(1).ecx & (1 << 26)) != 0,
        "XSAVE is not supported by the CPU"
    );

    // Disable FPU emulation and enable SSE and AVX instructions and exceptions.
    Cr0::enable(Cr0::MP);
    Cr0::disable(Cr0::EM);
    Cr4::enable(Cr4::OSFXSR | Cr4::OSXSAVE | Cr4::OSXMMEXCPT);

    // Enable legacy x87 FPU and SSE instructions.
    XCr0::enable(XCr0::X87 | XCr0::SSE);
}

/// Save the current FPU state into the given state buffer. Previous state stored
/// in the buffer will be overwritten.
///
/// # Safety
/// This function is unsafe because it assume that the buffer is large enough to
/// store the FPU state. If it is not the case, it will lead to undefined behavior
pub unsafe fn save(state: &mut State) {
    core::arch::x86_64::_xsave64(state.as_mut_ptr(), u64::MAX);
}

/// Restore the given FPU state from the given state buffer.
///
/// # Safety
/// This function is unsafe because it directly touches to the state of the FPU. It
/// assumes that the given state is valid. If it is not the case, then it may lead
/// to undefined behavior (likely an exception or a crash)
pub unsafe fn restore(state: &State) {
    core::arch::x86_64::_xrstor64(state.as_ptr(), u64::MAX);
}
