#![no_std]

extern crate alloc;

fn main() {
    let id = iron::syscall::task::spawn("/shell.elf").expect("Failed to load shell");
    iron::syscall::serial::print(&alloc::format!("Spawned shell with id: {}\n", id.0));

    let fd = iron::syscall::vfs::open("/test.txt", iron::syscall::vfs::OpenFlags::MUST_CREATE)
        .expect("Failed to open file");
    iron::syscall::serial::print(&alloc::format!("Opened file with fd: {:?}\n", fd));

    iron::syscall::vfs::close(fd).expect("Failed to close file");
}
