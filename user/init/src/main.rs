#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    for _ in 0..8 {
        iron::serial::print("I'm nice to others!\n");
        iron::task::yields();
    }

    iron::task::sleep(iron::task::id());
    iron::exit(iron::task::id() as i32);
}
