use super::SyscallResult;

pub fn task_exit(code: u64) -> SyscallResult {
    log::debug!("Task exit with code {}", code);
    SyscallResult::Ok
}
