#![no_std]

use alloc::Allocator;

pub mod alloc;
pub mod syscall;

#[global_allocator]
static mut ALLOCATOR: Allocator = Allocator::empty();

pub fn init() {}

#[panic_handler]
pub fn panic(_: &core::panic::PanicInfo) -> ! {
    syscall::serial::print("Application panicked");
    syscall::task::exit(-1);
}
