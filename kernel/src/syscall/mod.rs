use macros::syscall_handler;

pub mod task;

/// The return value of a syscall. A syscall can return any value that fits in an register: u64
/// and i64 on x86-64. However, values between -1 and -4095 for an i64 or `0xFFFF_FFFF_FFFF_F000`
/// and `0xFFFF_FFFF_FFFF_FFFF` for an u64 are reserved for indicating an error, This works
/// similarly to errno in Linux. This structure allow to greately simplify the code of the syscall
/// handlers when they must return an error or a success value.
pub struct SyscallReturn(i64);

impl SyscallReturn {
    /// Create a new `SyscallReturn` from a u64. It can be any value that fits in an u64, but
    /// values between `0xFFFF_FFFF_FFFF_F000` and `0xFFFF_FFFF_FFFF_FFFF` are reserved for
    /// indicating an error.
    ///
    /// # Panics
    /// This function panics if the value is between `0xFFFF_FFFF_FFFF_F000` and
    /// `0xFFFF_FFFF_FFFF_FFFF`
    pub fn new<T: Into<u64>>(value: T) -> Self {
        Self::from(value.into())
    }

    /// Indicate that the syscall failed with a code indicated by the `SyscallError` enum.
    /// The error code is converted to its negative value to fit in an i64 and is between
    /// -1 and -4095.
    #[must_use]
    pub fn failure(error: SyscallError) -> Self {
        Self(error.errno())
    }

    /// Indicate that the syscall succeeded, and simply return 0. It should be used when
    /// the syscall does not return any meaningful value.
    #[must_use]
    pub fn success() -> Self {
        Self(0)
    }
}

impl From<SyscallError> for SyscallReturn {
    fn from(error: SyscallError) -> Self {
        Self(error.errno())
    }
}

impl From<u64> for SyscallReturn {
    #[allow(clippy::cast_possible_wrap)]
    fn from(value: u64) -> Self {
        Self::from(value as i64)
    }
}

impl From<i64> for SyscallReturn {
    fn from(value: i64) -> Self {
        assert!(value >= 0 || value <= -4096);
        Self(value)
    }
}

/// A struct that contains all the possible syscall errors. When a syscall returns, it can
/// return any value that fit in an i64, but values between -1 and -4095 are reserved for
/// indicating an error. This works similarly to errno in Linux.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i64)]
pub enum SyscallError {
    NoSuchSyscall = 1,
    InvalidArgument = 2,
    TaskNotFound = 3,
    TaskInUse = 4,
}

impl SyscallError {
    /// Return the errno value of the current error. A syscall can return any value that
    /// fits in an i64, but values between -1 and -4095 are reserved for indicating an
    /// error, so we just convert the error to its negative value. This works similarly
    /// to errno in Linux.
    #[must_use]
    pub fn errno(&self) -> i64 {
        -(*self as i64)
    }
}

// A struct that contains all the syscall numbers used by the kernel.
#[non_exhaustive]
#[repr(u64)]
pub enum Syscall {
    TaskExit = 0,
    TaskDestroy = 1,
    TaskHandle = 2,
    Last,
}

impl Syscall {
    /// Create a new Syscall from a u64. If the u64 is not a valid syscall number, it
    /// returns None.
    #[must_use]
    pub fn from(id: u64) -> Option<Syscall> {
        if id < Self::Last as u64 {
            Some(unsafe { core::mem::transmute(id) })
        } else {
            None
        }
    }
}

/// Handle a syscall. This function is called from the syscall interrupt handler, written in
/// assembly and is responsible for dispatching the syscall to the appropriate handler within
/// the kernel.
#[syscall_handler]
#[allow(unused_variables)]
fn syscall(id: u64, a: u64, b: u64, c: u64, d: u64, e: u64) -> i64 {
    match Syscall::from(id) {
        Some(Syscall::TaskExit) => task::exit(a),
        Some(Syscall::TaskDestroy) => task::destroy(a),
        Some(Syscall::TaskHandle) => task::handle(),

        Some(Syscall::Last) | None => SyscallReturn::failure(SyscallError::NoSuchSyscall),
    }
    .0
}
