#![no_std]

extern crate alloc;

fn main() {
    let id = iron::syscall::task::spawn("/shell.elf").expect("Failed to load shell");
    iron::syscall::serial::print(&alloc::format!("Spawned task with id: {}\n", id.0));
}
