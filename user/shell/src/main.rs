fn main() {
    println!("Changing cwd to /bin");

    let start = syscall::clock::get_time().expect("get_time failed");
    syscall::vfs::change_cwd("/bin").expect("change_cwd failed");
    let end = syscall::clock::get_time().expect("get_time failed");

    let start_ns = start.seconds * 1_000_000_000 + start.nanoseconds;
    let end_ns = end.seconds * 1_000_000_000 + end.nanoseconds;

    println!("Took {} nanoseconds", end_ns - start_ns);
    
    let mut buffer = [0u8; 1024];
    let size = syscall::vfs::get_cwd(&mut buffer).expect("get_cwd failed");

    println!("cwd: {}", core::str::from_utf8(&buffer[..size]).unwrap());
}
