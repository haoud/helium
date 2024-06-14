use super::{syscall_return, Errno, Syscall};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum WriteError {
    NoSuchSyscall = 1,

    /// There is no serial port on the system
    NoSerialPort,

    /// The buffer is not in the user address space or the buffer is not
    /// writable
    BadAddress,

    UnknownError,
}

impl From<Errno> for WriteError {
    fn from(error: Errno) -> Self {
        if error.code() > -(Self::UnknownError as isize) {
            unsafe { core::mem::transmute(error) }
        } else {
            Self::UnknownError
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ReadError {
    NoSuchSyscall = 1,

    /// There is no serial port on the system
    NoSerialPort,

    /// The buffer is not in the user address space or the buffer is not
    /// writable
    BadAddress,

    UnknownError,
}

impl From<Errno> for ReadError {
    fn from(error: Errno) -> Self {
        if error.code() > -(Self::UnknownError as isize) {
            unsafe { core::mem::transmute(error) }
        } else {
            Self::UnknownError
        }
    }
}

/// Print a string to the serial port. This is a temporary function until we
/// have a proper way to print to the screen.
pub fn print(str: &str) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::SerialWrite as u64,
            in("rsi") str.as_ptr(),
            in("rdx") str.len(),
        );
    }
}

/// Write data to the serial port.
pub fn write(data: &[u8]) -> Result<usize, WriteError> {
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::SerialWrite as u64,
            in("rsi") data.as_ptr(),
            in("rdx") data.len(),
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(WriteError::from(errno)),
        Ok(size) => Ok(size as usize),
    }
}

/// Read data from the serial port.
pub fn read(buffer: &mut [u8]) -> Result<usize, ReadError> {
    let ret;

    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") Syscall::SerialRead as u64,
            in("rsi") buffer.as_mut_ptr(),
            in("rdx") buffer.len(),
            lateout("rax") ret,
        );
    }

    match syscall_return(ret) {
        Err(errno) => Err(ReadError::from(errno)),
        Ok(size) => Ok(size as usize),
    }
}
