pub mod mmu;
pub mod serial;
pub mod task;
pub mod vfs;
pub mod video;

/// The type of the return value of a syscall. All syscalls must return a value that fits
/// in an usize. However, some values are reserved for indicating an error: values between
/// -1 and -4095 are reserved for indicating an error (see `SyscallError` for more details).
pub type SyscallValue = usize;

// A struct that contains all the syscall numbers used by the kernel.
#[non_exhaustive]
#[repr(u64)]
pub enum Syscall {
    TaskExit = 0,
    TaskId = 1,
    TaskSleep = 2,
    TaskYield = 3,
    TaskSpawn = 4,
    SerialRead = 5,
    SerialWrite = 6,
    MmuMap = 7,
    MmuUnmap = 8,
    VideoFramebufferInfo = 9,
    VfsOpen = 10,
    VfsClose = 11,
    VfsRead = 12,
    VfsWrite = 13,
    VfsSeek = 14,
    VfsGetCwd = 15,
    VfsChangeCwd = 16,
}

impl Syscall {
    /// Create a new Syscall from a u64. If the u64 is not a valid syscall number, it
    /// returns None.
    #[must_use]
    pub fn from(id: usize) -> Option<Syscall> {
        match id {
            0 => Some(Self::TaskExit),
            1 => Some(Self::TaskId),
            2 => Some(Self::TaskSleep),
            3 => Some(Self::TaskYield),
            4 => Some(Self::TaskSpawn),
            5 => Some(Self::SerialRead),
            6 => Some(Self::SerialWrite),
            7 => Some(Self::MmuMap),
            8 => Some(Self::MmuUnmap),
            9 => Some(Self::VideoFramebufferInfo),
            10 => Some(Self::VfsOpen),
            11 => Some(Self::VfsClose),
            12 => Some(Self::VfsRead),
            13 => Some(Self::VfsWrite),
            14 => Some(Self::VfsSeek),
            15 => Some(Self::VfsGetCwd),
            16 => Some(Self::VfsChangeCwd),
            _ => None,
        }
    }
}

pub trait Errno {
    fn errno(&self) -> isize;
}

/// Handle a syscall. This function is called from the syscall interrupt handler, written in
/// assembly and is responsible for dispatching the syscall to the appropriate handler within
/// the kernel.
#[syscall_handler]
#[allow(unused_variables)]
#[allow(clippy::cast_possible_wrap)]
fn syscall(id: usize, a: usize, b: usize, c: usize, d: usize, e: usize) -> isize {
    let result: Result<usize, isize> = match Syscall::from(id) {
        Some(Syscall::TaskExit) => task::exit(a),
        Some(Syscall::TaskId) => task::id(),
        Some(Syscall::TaskSleep) => task::sleep(a),
        Some(Syscall::TaskYield) => task::yields(),
        Some(Syscall::TaskSpawn) => task::spawn(a).map_err(Into::into),
        Some(Syscall::SerialRead) => serial::read(a, b).map_err(Into::into),
        Some(Syscall::SerialWrite) => serial::write(a, b).map_err(Into::into),
        Some(Syscall::MmuMap) => mmu::map(a, b, c, d).map_err(Into::into),
        Some(Syscall::MmuUnmap) => mmu::unmap(a, b).map_err(Into::into),
        Some(Syscall::VideoFramebufferInfo) => video::framebuffer_info(a).map_err(Into::into),
        Some(Syscall::VfsOpen) => vfs::open(a, b).map_err(Into::into),
        Some(Syscall::VfsClose) => vfs::close(a).map_err(Into::into),
        Some(Syscall::VfsRead) => vfs::read(a, b, c).map_err(Into::into),
        Some(Syscall::VfsWrite) => vfs::write(a, b, c).map_err(Into::into),
        Some(Syscall::VfsSeek) => vfs::seek(a, b, c).map_err(Into::into),
        Some(Syscall::VfsGetCwd) => vfs::get_cwd(a, b).map_err(Into::into),
        Some(Syscall::VfsChangeCwd) => vfs::change_cwd(a).map_err(Into::into),
        None => Err(-1), // NoSuchSyscall,
    };

    #[cfg(feature = "trace-syscalls")]
    {
        if let Ok(value) = result {
            log::debug!("syscall {} successed -> {}", id, value);
        } else if let Err(error) = result {
            log::debug!("syscall {} failed -> {}", id, error);
        }
    }

    match result {
        Err(error) => error,
        Ok(value) => value as isize,
    }
}
