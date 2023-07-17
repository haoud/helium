#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    iron::task::sleep(iron::task::id());
    iron::exit(iron::task::id() as i32);
}
