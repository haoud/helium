#![no_std]

core::arch::global_asm!(include_str!("syscall.asm"));

extern "C" {
    pub fn exit(code: i32) -> !;
}

#[panic_handler]
pub fn panic(_: &core::panic::PanicInfo) -> ! {
    // SAFETY: This is safe because we are in a panic handler, so we can
    // exit here without any worries, because the application is already
    // in an invalid state.
    unsafe {
        exit(-1);
    }
}
