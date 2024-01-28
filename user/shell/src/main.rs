fn main() {
    println!("Creating /test");
    syscall::vfs::mkdir("/test").expect("mkdir failed");

    println!("Changing cwd to /test");
    syscall::vfs::change_cwd("/test").expect("change_cwd failed");

    let mut buffer = [0u8; 1024];
    let size = syscall::vfs::get_cwd(&mut buffer).expect("get_cwd failed");

    println!("cwd: {}", core::str::from_utf8(&buffer[..size]).unwrap());

    // Benchmark the get_pid syscall using the rdtsc
    let mut start: u64;
    let mut end: u64;
    let mut total: u64 = 0;

    for _ in 0..1_000_000 {
        start = unsafe { core::arch::x86_64::_rdtsc() };
        syscall::task::id();
        end = unsafe { core::arch::x86_64::_rdtsc() };
        total += end - start;
    }

    println!("Average get_pid syscall time: {} cycles", total / 1_000_000);

    /// Convert cycles to nanoseconds, assuming a 4.2 GHz CPU
    println!(
        "Average get_pid syscall time: {} ns",
        (total / 1_000_000) * 1000 / 4200
    );
}
