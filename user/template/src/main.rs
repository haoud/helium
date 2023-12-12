#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    iron::syscall::task::exit(0);
}
