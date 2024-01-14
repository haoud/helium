#![no_std]

use iron::syscall::*;
extern crate alloc;

fn main() {
    let fd = vfs::open(
        "/test.txt",
        vfs::OpenFlags::READ | vfs::OpenFlags::WRITE | vfs::OpenFlags::MUST_CREATE,
    )
    .expect("Failed to open file");
    serial::print(&alloc::format!("Opened file with fd: {:?}\n", fd));

    vfs::write(&fd, "Hello, world !".as_bytes()).expect("Failed to write to file");
    assert!(vfs::seek(&fd, vfs::Whence::Start(0)) == Ok(0));

    let mut buffer = [0u8; 1024];
    let bytes_read = vfs::read(&fd, &mut buffer).expect("Failed to read from file");
    let slice = &buffer[..bytes_read];

    serial::print(&alloc::format!(
        "Read {} bytes from test.txt: '{}'\n",
        bytes_read,
        alloc::string::String::from_utf8(slice.to_vec()).unwrap()
    ));

    vfs::close(fd).expect("Failed to close file");
} 
