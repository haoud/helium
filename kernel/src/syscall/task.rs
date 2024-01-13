use crate::{
    time::{timer::Timer, units::Nanosecond, uptime_fast},
    user::{
        self,
        scheduler::{self, Scheduler, SCHEDULER},
        string::SyscallString,
        task, vmm,
    },
    vfs,
};
use alloc::vec;

/// Exit the current task with the given exit code. The task will be terminated and
/// will not be scheduled again. If there is no other reference to the task, it will
/// be deallocated.
///
/// # Panics
/// This function panics if the current task is rescheduled after it has exited.
pub fn exit(code: usize) -> ! {
    log::debug!(
        "Task {} exited with code {}",
        SCHEDULER.current_task().id(),
        code
    );
    scheduler::terminate(code as u64);
    unsafe { SCHEDULER.schedule() };
    unreachable!("Task should never be scheduled again after exiting");
}

/// Return the identifier of the current task.
///
/// # Errors
/// This function will never return an error, but it is declared as returning a `Result` to
/// be consistent with the other syscalls.
///
/// # Panics
/// This function panics if there is no current task running on the CPU (which should
/// never happen and is a bug).
#[allow(clippy::cast_possible_truncation)]
pub fn id() -> Result<usize, isize> {
    Ok(SCHEDULER.current_task().id().0 as usize)
}

/// Put the current task to sleep for at least the given number of nanoseconds. The task
/// will be woken up when the timer expires. Due to the way the timer system works, the
/// task may not be woken up immediately after the timer expires and may be delayed by
/// a few milliseconds.
///
/// # Errors
/// This function will never return an error, but it is declared as returning a `Result`
/// to be consistent with the other syscalls. It always returns `0`.
pub fn sleep(nano: usize) -> Result<usize, isize> {
    let expiration = uptime_fast() + Nanosecond::new(nano as u64);

    // Create a timer that will wake up the task when it expires.
    let current = SCHEDULER.current_task();
    let timer = Timer::new(expiration, move |_| {
        if current.state() == task::State::Blocked {
            current.change_state(task::State::Ready);
        }
    });

    // Put the task to sleep if the timer is active (i.e nanoseconds > 0)
    if timer.active() {
        task::sleep();
    }
    Ok(0)
}

/// Yield the CPU to another task. If there is no other task ready to run or if there
/// is only lower priority tasks, the current task will continue to run.
///
/// # Errors
/// This function will never return an error, but it is declared as returning a `Result`
/// to be consistent with the other syscalls. It always returns `0`.
pub fn yields() -> Result<usize, isize> {
    unsafe { scheduler::yield_cpu() };
    Ok(0)
}

/// Spawn a new task from the given ELF file. The ELF file must be a statically linked
/// executable. The ELF file will be loaded into memory and the task will be created.
/// The task will be put in the ready queue and will be scheduled to run as soon as
/// possible.
///
/// # Errors
/// TODO:
///
/// # Panics
/// TODO:
///
/// # Optimization
/// Currently, the whole ELF file is read into memory before being loaded. This is
/// inefficient and should be changed to map the file into memory and load it on
/// demand during the execution of the task.
pub fn spawn(path: usize) -> Result<usize, SpawnError> {
    let ptr = user::Pointer::<SyscallString>::from_usize(path).ok_or(SpawnError::BadAddress)?;
    let path = user::String::from_raw_ptr(&ptr)
        .ok_or(SpawnError::BadAddress)?
        .fetch()
        .map_err(|_| SpawnError::BadAddress)?;

    // Read all the elf file into memory
    let current_task = SCHEDULER.current_task();
    let inode = vfs::lookup(
        &path,
        &current_task.root().lock(),
        &current_task.cwd().lock(),
    )
    .map_err(|e| match e {
        vfs::LookupError::NotFound(_, _) => todo!(),
        vfs::LookupError::InvalidPath(_) => todo!(),
        vfs::LookupError::NotADirectory => todo!(),
        vfs::LookupError::CorruptedFilesystem => todo!(),
        vfs::LookupError::IoError => SpawnError::IoError,
    })?;

    let len = inode.state.lock().size;
    let file = vfs::file::OpenFile::new(vfs::file::OpenFileCreateInfo {
        operation: inode.file_ops.clone(),
        inode: inode,
        open_flags: vfs::file::OpenFlags::READ,
        data: Box::new(()),
    });

    // Read all the file
    let mut data = vec![0; len as usize].into_boxed_slice();
    let readed = file
        .as_file()
        .unwrap()
        .read(&file, &mut data, vfs::file::Offset(0))
        .map_err(|_| SpawnError::IoError)?;

    if readed != len as usize {
        log::debug!("Readed {} bytes, expected {}", readed, len);
        return Err(SpawnError::IoError);
    }

    let task = task::elf::load(Arc::new(Spinlock::new(vmm::Manager::new())), &data)
        .expect("Failed to load task");

    let id = task.id();
    SCHEDULER.add_task(task);

    Ok(id.0 as usize)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum SpawnError {
    NoSuchSyscall = 1,
    BadAddress,
    IoError,
    InvalidElf,
    OutOfMemory,
    UnknownError,
}

impl From<SpawnError> for isize {
    fn from(error: SpawnError) -> Self {
        -(error as isize)
    }
}
