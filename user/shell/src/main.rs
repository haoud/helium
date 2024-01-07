#![no_std]

extern crate alloc;

fn main() {
    iron::syscall::serial::print("Hello, world!\n");
}
