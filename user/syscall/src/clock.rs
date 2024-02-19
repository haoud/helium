use super::{syscall_return, Errno, Syscall};

/// The timespec struct is used to represent time in seconds and nanoseconds.
#[repr(C)]
pub struct Timespec {
    pub seconds: u64,
    pub nanoseconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum GetTimeError {
    /// The syscall number is invalid.
    NoSuchSyscall = 1,

    /// An invalid file descriptor was passed as an argument
    BadAddress,

    /// An unknown error occurred
    UnknownError,
}

impl From<Errno> for GetTimeError {
    fn from(error: Errno) -> Self {
        if error.code() > -(Self::UnknownError as isize) {
            unsafe { core::mem::transmute(error) }
        } else {
            Self::UnknownError
        }
    }
}

/// Get the current time since the Unix epoch.
///
/// # Errors
/// - `GetTimeError::BadAddress`: The given address for the timespec is not a valid address.
/// This should never happen, as the address is always valid and directly provided by this
/// function.
pub fn get_time() -> Result<Timespec, GetTimeError> {
    let mut timespec = Timespec {
        seconds: 0,
        nanoseconds: 0,
    };

    let ret: usize;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::ClockGetTime as u64,
            in("rsi") &mut timespec as *mut Timespec as u64,
            lateout("rax") ret,
        );
    }

    // Transmute the return value to ReadInfoError if a valid error code was returned.
    // If the error code is unknown, return an UnknownError.
    match syscall_return(ret) {
        Err(errno) => Err(GetTimeError::from(errno)),
        Ok(_) => Ok(timespec),
    }
}
