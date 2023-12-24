#![no_std]
#![no_main]

extern crate alloc;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    iron::init();
    iron::syscall::serial::print("Hello, world!\n");

    // Test dynamic allocation
    let data = alloc::vec![0u8; 1024*1024*2];
    iron::syscall::serial::print(&alloc::format!(
        "Data dynamically allocated at {:p}\n",
        data.as_ptr()
    ));

    // Exit the task
    iron::syscall::task::exit(iron::syscall::task::id() as i32);
}
