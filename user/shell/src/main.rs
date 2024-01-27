fn main() {
    println!("Changing cwd to /bin");
    syscall::vfs::change_cwd("/bin").expect("change_cwd failed");

    let mut buffer = [0u8; 1024];
    let size = syscall::vfs::get_cwd(&mut buffer).expect("get_cwd failed");

    println!("cwd: {}", core::str::from_utf8(&buffer[..size]).unwrap());
}
