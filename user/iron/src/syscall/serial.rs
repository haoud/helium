use super::Syscall;

/// Print a string to the serial port. This is a temporary function until we have
/// a proper way to print to the screen.
pub fn print(str: &str) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::SerialWrite as u64,
            in("rsi") str.as_ptr(),
            in("rdx") str.len(),
        );
    }
}
