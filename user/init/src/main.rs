#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut x: u64 = 0;
    for _ in 0..500000 {
        x = core::hint::black_box(iron::task::id());
    }

    iron::serial::print("Goodbye, cruel world !\n");
    iron::exit(x as i32);
}
