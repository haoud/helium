use crate::{
    logger::SERIAL,
    user::buffer::{BufferError, UserStandardBuffer},
};
use addr::user::InvalidUserVirtual;

/// Write data to the serial port and return the number of bytes written.
/// This functions is very basic and does not support any kind of timeout
/// nor does it support serial ports other than the first one. It does not
/// even detect if the serial port is prsent !
/// FIXME: This should obviously be fixed.
///
/// # Errors
/// - `SyscallError::BadAddress`: the buffer is not in the user address space
///    or the buffer is not readable.
pub fn write(buffer: usize, len: usize) -> Result<usize, WriteError> {
    let mut buffer = UserStandardBuffer::new(buffer, len)?;
    let serial = SERIAL.lock();
    while let Some(buf) = buffer.read_buffered() {
        buf.iter().for_each(|&byte| serial.write(byte));
    }

    Ok(buffer.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum WriteError {
    NoSuchSyscall = 1,

    /// The buffer is not in the user address space or the buffer is not readable
    BadAddress,

    UnknownError,
}

impl From<BufferError> for WriteError {
    fn from(e: BufferError) -> Self {
        match e {
            BufferError::NotInUserSpace => Self::BadAddress,
        }
    }
}

impl From<InvalidUserVirtual> for WriteError {
    fn from(_: InvalidUserVirtual) -> Self {
        Self::BadAddress
    }
}

impl From<WriteError> for isize {
    fn from(error: WriteError) -> Self {
        -(error as isize)
    }
}

/// Read data from the serial port. The function will block until some data
/// is available, and the function will return the number of bytes read.
///
/// # Warning
/// The function is very simple and does not support any kind of timeout.
/// The caller can block indefinitely if no data is available or even worse,
/// if the serial port is not present ! Even worse, the thread will be blocked
/// in an active busy loop, consuming CPU cycles and preventing any preemption
/// in the core !
/// FIXME: This should obviously be fixed.
///
/// # Errors
/// - `SyscallError::BadAddress`: the buffer is not in the user address space
///   or the buffer is not writable.
pub fn read(buffer: usize, size: usize) -> Result<usize, ReadError> {
    let mut buffer = UserStandardBuffer::new(buffer, size)?;
    let serial = SERIAL.lock();

    while buffer.write(serial.read()).is_some() {}

    Ok(buffer.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ReadError {
    NoSuchSyscall = 1,

    /// There is no serial port on the system
    NoSerialPort,

    /// The buffer is not in the user address space or the buffer is not writable
    BadAddress,

    UnknownError,
}

impl From<BufferError> for ReadError {
    fn from(e: BufferError) -> Self {
        match e {
            BufferError::NotInUserSpace => Self::BadAddress,
        }
    }
}

impl From<InvalidUserVirtual> for ReadError {
    fn from(_: InvalidUserVirtual) -> Self {
        Self::BadAddress
    }
}

impl From<ReadError> for isize {
    fn from(error: ReadError) -> Self {
        -(error as isize)
    }
}
