#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut x: u64 = 0;
    for i in 0..5000000 {
        x = core::hint::black_box(i);
    }

    _ = x;
    iron::exit(iron::task::id() as i32);
}
