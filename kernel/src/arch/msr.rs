/// Represents an MSR register. For more information about MSR registers, see
/// the Intel manual, volume 4, chapter 2.
pub struct Register(u32);

impl Register {
    pub const EFER: Self = Self(0xC0000080);
    pub const STAR: Self = Self(0xC0000081);
    pub const LSTAR: Self = Self(0xC0000082);
    pub const CSTAR: Self = Self(0xC0000083);
    pub const FMASK: Self = Self(0xC0000084);
    pub const FS_BASE: Self = Self(0xC0000100);
    pub const GS_BASE: Self = Self(0xC0000101);
    pub const KERNEL_GS_BASE: Self = Self(0xC0000102);
}

/// Write the given value to the given MSR.
///
/// # Safety
/// This function is unsafe because writing to an MSR can cause unexpected side effects and
/// potentially violate memory safety. It can also cause undefined behavior if the MSR is not
/// supported by the CPU.
pub unsafe fn write(msr: Register, value: u64) {
    core::arch::asm!("wrmsr", in("ecx") msr.0, in("eax") (value as u32), in("edx") (value >> 32));
}

/// Read the current value of the given MSR.
///
/// # Safety
/// This function is unsafe because reading from an MSR can cause unexpected side effects and
/// potentially violate memory safety. It can also cause undefined behavior if the MSR is not
/// supported by the
pub unsafe fn read(msr: Register) -> u64 {
    let low: u32;
    let high: u32;
    core::arch::asm!("rdmsr", in("ecx") msr.0, out("eax") low, out("edx") high);
    (high as u64) << 32 | (low as u64)
}
