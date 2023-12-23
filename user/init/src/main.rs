#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    iron::syscall::serial::print("Hello, world!\n");
    iron::syscall::task::exit(iron::syscall::task::id() as i32);
}
