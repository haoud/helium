/// Exit QEMU with the given exit code. The returned exit code to the caller will be
/// `(code << 1) | 1.`
///
/// # Safety
/// This function is used I/O port access to exit QEMU, which could result in undefined behavior
/// if the kernel is not running in QEMU.
pub unsafe fn exit(code: u32) -> ! {
    x86_64::instruction::outd(0x501, code);
    x86_64::cpu::freeze();
}
