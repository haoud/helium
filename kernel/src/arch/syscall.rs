use super::msr;
core::arch::global_asm!(include_str!("asm/syscall.asm"));

extern "C" {
    fn syscall_enter();
}

/// Set up the system call mechanism. This function should be called once during the kernel
/// initialization.
///
/// # Safety
/// This function is unsafe because it assume that the `syscall_enter` function exists in the
/// kernel code, and is correctly set up to handle system calls.
pub unsafe fn setup() {
    // The Star MSR is used to set up the kernel segment base in bits 47:32, and the user
    // segment base in bits 63:48. The first 32 bits are not used in 64-bit mode.
    msr::write(msr::Register::STAR, 0x0018_0008_0000_0000);

    // The LStar MSR is used to set up the entry point for system calls. This is the address
    // of the entry function.
    msr::write(msr::Register::LSTAR, syscall_enter as usize as u64);

    // The SFMask MSR is used to set up mask applied to the RFLAGS register when a system
    // call is made. Currently, we mask out the interrupt flag, so that interrupts are
    // disabled during system calls, and the direction flag (required by System V ABI).
    msr::write(msr::Register::FMASK, 0x0000_0000_0000_0202);

    // Enable the Sytem Call Extension (bit 0 of the EFER MSR), allowing the use of the
    // SYSCALL/SYSRET instructions.
    msr::write(msr::Register::EFER, msr::read(msr::Register::EFER) | 0x01);
}
