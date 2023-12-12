#![no_std]

pub mod syscall;

#[panic_handler]
pub fn panic(_: &core::panic::PanicInfo) -> ! {
    syscall::serial::print("Application panicked");
    syscall::task::exit(-1);
}
