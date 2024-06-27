use crate::{
    time::{
        self,
        units::{Nanosecond, Second},
    },
    user,
};

#[repr(C)]
pub struct Timespec {
    pub seconds: u64,
    pub nanoseconds: u64,
}

/// Get the clock monotonic time. This is the time since the kernel booted.
///
/// # Errors
/// See [`GetTimeError`] for details.
pub fn get_time(buffer: usize) -> Result<usize, GetTimeError> {
    let ptr = user::Pointer::new(buffer as *mut Timespec)
        .ok_or(GetTimeError::BadAddress)?;

    let time = time::uptime();
    let second = Second::from(time);
    let nano = time - Nanosecond::from(second);

    let time = Timespec {
        seconds: second.0,
        nanoseconds: nano.0,
    };

    // SAFETY: `ptr` is a valid pointer to a [`Timestamp`] object.
    unsafe {
        user::Object::write(&ptr, &time);
    }
    Ok(0)
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

impl From<GetTimeError> for isize {
    fn from(error: GetTimeError) -> Self {
        -(error as isize)
    }
}
