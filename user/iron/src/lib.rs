#![no_std]

pub mod serial;
pub mod task;

/// Terminates the current process with the specified exit code. This function will never
/// return and will immediately terminate the current process. Because this function never
/// returns, and that it terminates the process, no destructors on the current stack will
/// be run.
pub fn exit(code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 0,
            in("rsi") code,
            options(noreturn)
        );
    }
}

#[panic_handler]
pub fn panic(_: &core::panic::PanicInfo) -> ! {
    exit(-1);
}
