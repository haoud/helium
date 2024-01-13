use crate::{
    logger::SERIAL,
    user::buffer::{BufferError, UserStandardBuffer},
};
use addr::user::{InvalidUserVirtual, UserVirtual};

/// Write data to the serial port.
///
/// # Errors
/// - `SyscallError::BadAddress`: the buffer is not in the user address space.
pub fn write(buffer: usize, len: usize) -> Result<usize, WriteError> {
    let address = UserVirtual::try_new(buffer)?;
    let mut buffer = UserStandardBuffer::try_new(address, len)?;

    let serial = SERIAL.lock();
    while let Some(buf) = buffer.read_buffered() {
        buf.iter().for_each(|&byte| serial.write(byte));
    }

    Ok(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum WriteError {
    NoSuchSyscall = 1,
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

/// Read data from the serial port.
///
/// # Errors
/// This function is not implemented for now, and will always return `SyscallError::NotImplemented`
/// when called.
pub fn read(_: usize, _: usize) -> Result<usize, ReadError> {
    Err(ReadError::NotImplemented)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(usize)]
pub enum ReadError {
    NoSuchSyscall = 1,
    NotImplemented,
    UnknownError,
}

impl From<ReadError> for isize {
    fn from(error: ReadError) -> Self {
        -(error as isize)
    }
}
