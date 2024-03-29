pub mod clock;
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
    ClockGetTime = 9,
    VideoFramebufferInfo = 10,
    VfsOpen = 11,
    VfsClose = 12,
    VfsRead = 13,
    VfsWrite = 14,
    VfsSeek = 15,
    VfsGetCwd = 16,
    VfsChangeCwd = 17,
    VfsMkdir = 18,
    VfsRmdir = 19,
    VfsUnlink = 20,
    VfsTruncate = 21,
    VfsStat = 22,
    VfsReaddir = 23,
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
            9 => Some(Self::ClockGetTime),
            10 => Some(Self::VideoFramebufferInfo),
            11 => Some(Self::VfsOpen),
            12 => Some(Self::VfsClose),
            13 => Some(Self::VfsRead),
            14 => Some(Self::VfsWrite),
            15 => Some(Self::VfsSeek),
            16 => Some(Self::VfsGetCwd),
            17 => Some(Self::VfsChangeCwd),
            18 => Some(Self::VfsMkdir),
            19 => Some(Self::VfsRmdir),
            20 => Some(Self::VfsUnlink),
            21 => Some(Self::VfsTruncate),
            22 => Some(Self::VfsStat),
            23 => Some(Self::VfsReaddir),
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
        Some(Syscall::ClockGetTime) => clock::get_time(a).map_err(Into::into),
        Some(Syscall::VideoFramebufferInfo) => video::framebuffer_info(a).map_err(Into::into),
        Some(Syscall::VfsOpen) => vfs::open(a, b, c).map_err(Into::into),
        Some(Syscall::VfsClose) => vfs::close(a).map_err(Into::into),
        Some(Syscall::VfsRead) => vfs::read(a, b, c).map_err(Into::into),
        Some(Syscall::VfsWrite) => vfs::write(a, b, c).map_err(Into::into),
        Some(Syscall::VfsSeek) => vfs::seek(a, b, c).map_err(Into::into),
        Some(Syscall::VfsGetCwd) => vfs::get_cwd(a, b).map_err(Into::into),
        Some(Syscall::VfsChangeCwd) => vfs::change_cwd(a).map_err(Into::into),
        Some(Syscall::VfsMkdir) => vfs::mkdir(a, b).map_err(Into::into),
        Some(Syscall::VfsRmdir) => vfs::rmdir(a, b).map_err(Into::into),
        Some(Syscall::VfsUnlink) => vfs::unlink(a, b).map_err(Into::into),
        Some(Syscall::VfsTruncate) => vfs::truncate(a, b).map_err(Into::into),
        Some(Syscall::VfsStat) => vfs::stat(a, b, c).map_err(Into::into),
        Some(Syscall::VfsReaddir) => vfs::readdir(a, b).map_err(Into::into),
        None => Err(-1), // NoSuchSyscall,
    };

    #[cfg(feature = "trace-syscalls")]
    {
        if let Ok(value) = result {
            log::trace!("syscall {} successed -> {}", id, value);
        } else if let Err(error) = result {
            log::trace!("syscall {} failed -> {}", id, error);
        }
    }

    match result {
        Err(error) => error,
        Ok(value) => value as isize,
    }
}
