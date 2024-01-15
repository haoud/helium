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

/// Write data to the serial port.
pub fn write(data: &[u8]) -> usize {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::SerialWrite as u64,
            in("rsi") data.as_ptr(),
            in("rdx") data.len(),
        );
    }
    data.len()
}

/// Read data from the serial port.
pub fn read(_: &mut [u8]) -> usize {
    0
}
