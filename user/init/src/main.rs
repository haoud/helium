#![no_std]
#![no_main]

#[no_mangle]
pub extern "C" fn _start() -> ! {
    iron::mmu::map(
        0x1000,
        4096,
        iron::mmu::Access::WRITE,
        iron::mmu::Flags::FIXED,
    )
    .unwrap();
    iron::mmu::map(
        0x2000,
        4096,
        iron::mmu::Access::WRITE,
        iron::mmu::Flags::FIXED,
    )
    .unwrap();
    iron::mmu::map(
        0x3000,
        4096,
        iron::mmu::Access::WRITE,
        iron::mmu::Flags::FIXED,
    )
    .unwrap();
    iron::mmu::unmap(0x2000, 4096).unwrap();

    iron::exit(
        iron::mmu::map(
            0x2000,
            4096,
            iron::mmu::Access::WRITE,
            iron::mmu::Flags::FIXED,
        )
        .unwrap_or(666) as i32,
    );

    for _ in 0..8 {
        iron::serial::print("I'm nice to others!\n");
        iron::task::yields();
    }

    iron::task::sleep(iron::task::id());
    iron::exit(iron::task::id() as i32);
}
