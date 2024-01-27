fn main() {
    println!("Creating /test");
    syscall::vfs::mkdir("/test").expect("mkdir failed");

    println!("Changing cwd to /test");
    syscall::vfs::change_cwd("/test").expect("change_cwd failed");

    let mut buffer = [0u8; 1024];
    let size = syscall::vfs::get_cwd(&mut buffer).expect("get_cwd failed");

    println!("cwd: {}", core::str::from_utf8(&buffer[..size]).unwrap());
}
