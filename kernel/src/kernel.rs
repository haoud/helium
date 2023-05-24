/// A enum that represents the stopping reason of the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stop {
    Success = 1,
    Failure = 2,
}

/// Stop the execution of the kernel. Depending on the features flags, it either closes the
/// emulator or freezes the CPU. This should be used when the kernel can't continue its execution,
/// or when the kernel has finished its execution.
///
/// # Safety
/// This function is unsafe because depending on some features flags, it either closes the emulator
/// or freezes the CPU, which could result in undefined behavior if the kernel is not running in
/// QEMU.
#[allow(unused_variables)]
pub unsafe fn stop(code: Stop) -> ! {
    cfg_if::cfg_if! {
        if #[cfg(feature = "test")] {
            crate::emulator::qemu::exit(code as u32);
        }
    }
    x86_64::cpu::freeze();
}
