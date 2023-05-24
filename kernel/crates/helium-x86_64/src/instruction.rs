/// Write an 8 bit value from a port.
///
/// # Safety
/// This function is unsafe because writing to a port can have side effects, including causing
/// the hardware to do something unexpected and possibly violating memory safety.
#[inline(always)]
pub unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!("out dx, al", in("dx") port, in("al") value);
}

/// Write an 16 bit value to a port.
///
/// # Safety
/// This function is unsafe because writing to a port can have side effects, including causing
/// the hardware to do something unexpected and possibly violating memory safety.
#[inline(always)]
pub unsafe fn outw(port: u16, value: u16) {
    core::arch::asm!("out dx, ax", in("dx") port, in("ax") value);
}

/// Write an 32 bit value to a port.
///
/// # Safety
/// This function is unsafe because writing to a port can have side effects, including causing
/// the hardware to do something unexpected and possibly violating memory safety.
#[inline(always)]
pub unsafe fn outd(port: u16, value: u32) {
    core::arch::asm!("out dx, eax", in("dx") port, in("eax") value);
}

/// Read an 8 bit value from a port.
///
/// # Safety
/// This function is unsafe because reading from a port can have side effects, including causing
/// the hardware to do something unexpected and possibly violating memory safety.
#[must_use]
#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let mut value: u8;
    core::arch::asm!("in al, dx", in("dx") port, out("al") value);
    value
}

/// Read an 16 bit value from a port.
///
/// # Safety
/// This function is unsafe because reading from a port can have side effects, including causing
/// the hardware to do something unexpected and possibly violating memory safety.
#[must_use]
#[inline(always)]
pub unsafe fn inw(port: u16) -> u16 {
    let mut value: u16;
    core::arch::asm!("in ax, dx", in("dx") port, out("ax") value);
    value
}

/// Read an 32 bit value from a port.
///
/// # Safety
/// This function is unsafe because reading from a port can have side effects, including causing
/// the hardware to do something unexpected and possibly violating memory safety.
#[must_use]
#[inline(always)]
pub unsafe fn ind(port: u16) -> u32 {
    let mut value: u32;
    core::arch::asm!("in eax, dx", in("dx") port, out("eax") value);
    value
}

/// Disable interrupts on the current CPU core.
///
/// # Safety
/// This function is unsafe because disabling interrupts can have side effects and can freeze
/// the computer if not used properly.
#[inline(always)]
pub unsafe fn cli() {
    core::arch::asm!("cli");
}

/// Enable interrupts on the current CPU core.
///
/// # Safety
/// This function is unsafe because enabling interrupts can have side effects and can lead to
/// a triple fault and a computer reboot if the interrupts are not properly handled.
#[inline(always)]
pub unsafe fn sti() {
    core::arch::asm!("sti");
}

/// Halt the CPU until the next interrupt arrives.
///
/// # Safety
/// This function is unsafe because halting the CPU can have side effects, especially if the
/// interrupts are not enabled (hang the CPU forever).
#[inline(always)]
pub unsafe fn hlt() {
    core::arch::asm!("hlt");
}

/// Improve the CPU performance of spinlock loops. The processor uses this hint to avoid the memory
/// order violation in most situations, which greatly improves processor performance
#[inline(always)]
pub fn pause() {
    unsafe {
        core::arch::asm!("pause");
    }
}

/// Load the given GDT register into the CPU. The parameter is a pointer to the
/// GDT register.
///
/// # Safety
/// This function is unsafe because it can cause undefined behavior if the given
/// gdtr is not a valid GDT register, if the GDT is not loaded or not properly configured...
#[inline(always)]
pub unsafe fn lgdt(gdtr: u64) {
    core::arch::asm!("lgdt [{}]", in(reg) gdtr, options(readonly, nostack, preserves_flags));
}

/// Load the given IDT register into the CPU. The parameter is a pointer to the
/// IDT register.
///
/// # Safety
/// This function is unsafe because it can cause undefined behavior if the given
/// idtr is not a valid IDT register, if the IDT is not loaded or not properly configured...
#[inline(always)]
pub unsafe fn lidt(idtr: u64) {
    core::arch::asm!("lidt [{}]", in(reg) idtr, options(readonly, nostack, preserves_flags));
}

/// Load a new task state segment (TSS) into the CPU. The parameter is the selector of the TSS.
///
/// # Safety
/// This function is unsafe because it can cause undefined behavior if the given selector is not a
/// valid TSS selector, if the TSS is not loaded or not properly configured or if the GDT is not
/// loaded or not properly configured.
#[inline(always)]
pub unsafe fn ltr(selector: u16) {
    core::arch::asm!("ltr ax", in("ax") selector, options(readonly, nostack, preserves_flags));
}

/// Invalidate the TLB entry for the given virtual address. This is useful when modifying the
/// page tables, but should be used with caution because it can dramatically reduce performances
/// if used too often.
#[inline(always)]
pub fn invlpg(addr: u64) {
    unsafe {
        core::arch::asm!("invlpg [{}]", in(reg) addr, options(nostack, preserves_flags));
    }
}
