fn main() {
    println!("init: spawning shell");
    syscall::task::spawn("/shell.elf").expect("Failed to spawn shell");
}
