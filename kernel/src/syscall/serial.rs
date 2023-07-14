use super::{SyscallError, SyscallReturn};
use crate::{logger::SERIAL, user::buffer::UserBuffered};
use addr::user::UserVirtual;
use usize_cast::IntoUsize;

#[must_use]
pub fn write(buffer: u64, len: u64) -> SyscallReturn {
    let address = UserVirtual::try_new(buffer);
    if let Ok(address) = address {
        let buffer = UserBuffered::try_new(address, len.into_usize());
        if let Some(buffer) = buffer {
            serial_write(buffer)
        } else {
            SyscallReturn::failure(SyscallError::BadAddress)
        }
    } else {
        SyscallReturn::failure(SyscallError::BadAddress)
    }
}

#[must_use]
pub fn read(_: u64, _: u64) -> SyscallReturn {
    SyscallReturn::failure(SyscallError::NotImplemented)
}

fn serial_write(mut buffer: UserBuffered) -> SyscallReturn {
    while let Some(buf) = buffer.read_buffered() {
        buf.iter().for_each(|&byte| SERIAL.lock().write(byte));
    }
    SyscallReturn::success()
}
