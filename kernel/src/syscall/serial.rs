use super::{SyscallError, SyscallValue};
use crate::{logger::SERIAL, user::buffer::UserBuffered};
use addr::user::UserVirtual;

/// Write data to the serial port.
///
/// # Errors
/// - `SyscallError::BadAddress`: the buffer is not in the user address space.
pub fn write(buffer: usize, len: usize) -> Result<SyscallValue, SyscallError> {
    let address = UserVirtual::try_new(buffer)?;
    let mut buffer = UserBuffered::try_new(address, len)?;

    let serial = SERIAL.lock();
    while let Some(buf) = buffer.read_buffered() {
        buf.iter().for_each(|&byte| serial.write(byte));
    }

    Ok(0)
}

/// Read data from the serial port.
///
/// # Errors
/// This function is not implemented for now, and will always return `SyscallError::NotImplemented`
/// when called.
pub fn read(_: usize, _: usize) -> Result<SyscallValue, SyscallError> {
    Err(SyscallError::NotImplemented)
}
