use macros::syscall_handler;

#[syscall_handler]
pub fn syscall(syscall: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
    log::info!("syscall: {}", syscall);
    0
}
