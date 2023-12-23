use crate::x86_64;
use addr::user::UserVirtual;

/// The default user buffer type that should be used in most cases unless you had a
/// specific reason to use a different buffer size.
pub type UserStandardBuffer = UserBuffer<{ crate::config::BUFFERED_LEN }>;

/// A user buffer of bytes. This buffer is used to read or write data from the user address
/// space. This buffer is backed by an internal buffer of size `N` bytes, used to read chunks
/// of data from the user address space without too much overhead in both CPU time and memory.
pub struct UserBuffer<const N: usize> {
    /// The start address of the buffer in the user address space.
    start: UserVirtual,

    /// The offset of the buffer in the user address space.
    offset: usize,

    /// The length of the buffer.
    len: usize,

    /// An temporary buffer used to copy data from the user address space to the kernel
    /// space without allocating a buffer as big as the user buffer to avoid memory
    /// exhaustion.
    buffer: [u8; N],
}

impl<const N: usize> UserBuffer<N> {
    /// Create a new user buffer from a start address and a length.
    ///
    /// # Panics
    /// Panic if a part of the buffer is not in the user address space.
    #[must_use]
    pub fn new(start: UserVirtual, len: usize) -> Self {
        match Self::try_new(start, len) {
            Err(BufferError::NotInUserSpace) => {
                panic!("UserBuffer: buffer not in user address space")
            }
            Ok(buffer) => buffer,
        }
    }

    /// Try to create a new user buffer from a start address and a length.
    ///
    /// # Errors
    /// Return an error if a part of the buffer is not in the user address space.
    pub fn try_new(start: UserVirtual, len: usize) -> Result<Self, BufferError> {
        if UserVirtual::is_user(start.as_usize() + len) {
            Ok(Self {
                start,
                offset: 0,
                len,
                buffer: [0; N],
            })
        } else {
            Err(BufferError::NotInUserSpace)
        }
    }

    /// Get the remaning bytes to read or the remaning space to write in the user buffer.
    #[must_use]
    pub fn remaning(&self) -> usize {
        self.len - self.offset
    }

    /// Get the current offset inside the user buffer.
    #[must_use]
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Get the length of the user buffer.
    #[must_use]
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Read one byte from the user buffer using an internal buffer and increment the offset
    /// inside the user buffer. This is very inefficient, so you should use [`read_buffered`]
    /// instead to read multiple bytes at once.
    ///
    /// # Returns
    /// - `Some(byte)` the byte readed from the user buffer.
    /// - `None` if the user buffer is empty.
    #[must_use]
    pub fn read(&mut self) -> Option<u8> {
        self.read_buffered().map(|buf| buf[0])
    }

    /// Write one byte to the user buffer and increment the offset inside the user buffer.
    /// This is very inefficient, so you should use [`write_buffered`] instead to write
    /// multiple bytes at once.
    ///
    /// # Returns
    /// - `Some(())` if the byte was written to the user buffer.
    /// - `None` if the user buffer is full.
    #[must_use]
    pub fn write(&mut self, byte: u8) -> Option<()> {
        self.write_buffered(&[byte])
    }

    /// Read a slice of bytes from the user buffer using an internal buffer and increment the
    /// offset inside the user buffer. If the remaning bytes to read is greater than the
    /// internal buffer size, then it will perform a partial read, and multiple calls to this
    /// function will be required to read the whole buffer.
    ///
    /// # Returns
    /// - `Some(slice)` a slice of bytes readed from the user buffer.
    /// - `None` if the user buffer is empty.
    #[must_use]
    pub fn read_buffered(&mut self) -> Option<&mut [u8]> {
        if self.offset >= self.len {
            return None;
        }

        // SAFETY: This is safe because we checked that the buffer is fully in the
        // user space. Since the buffer is in the user space, data races are allowed
        // and are the responsibility of the user to prevent them.
        unsafe {
            let src = self.start.as_ptr::<u8>().add(self.offset);
            let len = core::cmp::min(self.len - self.offset, N);
            let dst = self.buffer.as_mut_ptr();

            x86_64::user::copy_from(src, dst, len);
            self.offset += len;

            Some(self.buffer[0..len].as_mut())
        }
    }

    /// Write a slice of bytes to the user buffer and increment the offset inside the user buffer.
    /// If the slice is greater than the remaning space in the user buffer, then it returns `None`.
    ///
    /// # Returns
    ///  - `Some(())` if the slice was entirely written to the user buffer.
    /// - `None` if the user buffer is full.
    #[must_use]
    pub fn write_buffered(&mut self, buf: &[u8]) -> Option<()> {
        if self.offset + buf.len() >= self.len {
            return None;
        }

        // SAFETY: This is safe because we checked that the buffer is fully in the
        // user space. Since the buffer is in the user space, data races are allowed
        // and are the responsibility of the user to prevent them.
        unsafe {
            let dst = self.start.as_mut_ptr::<u8>().add(self.offset);
            let src = buf.as_ptr();
            let len = buf.len();

            x86_64::user::copy_to(src, dst, len);
            self.offset += len;
        }

        Some(())
    }
}

/// Represent an error that can occur when creating a user buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferError {
    /// The buffer is not in the user address space.
    NotInUserSpace,
}
