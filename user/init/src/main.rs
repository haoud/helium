#![no_std]
#![allow(internal_features)]
#![feature(prelude_import)]

#[prelude_import]
pub use iron::prelude::*;

#[macro_use]
extern crate iron;
extern crate alloc;

fn main() {
    let id = iron::syscall::task::spawn("/shell.elf").expect("Failed to load shell");
    println!("Spawned task with id: {}", id.0);

    let fd = iron::syscall::vfs::open("/test.txt", iron::syscall::vfs::OpenFlags::MUST_CREATE)
        .expect("Failed to open file");
    println!("Opened file with fd: {:?}", fd);
}
