pub fn print(str: &str) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 3,
            in("rsi") str.as_ptr(),
            in("rdx") str.len(),
        );
    }
}
