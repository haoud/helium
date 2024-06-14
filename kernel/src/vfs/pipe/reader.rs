use super::Pipe;

/// A pipe reader. This is a wrapper around a pipe that provides a safe
/// interface for reading from the pipe.
pub struct PipeReader {
    pipe: Arc<Pipe>,
}

impl PipeReader {
    #[must_use]
    pub fn new(pipe: Arc<Pipe>) -> Self {
        Self { pipe }
    }

    /// Reads a byte from the pipe. If the pipe is empty, the current thread
    /// will be put to sleep until a byte is available. After the byte is read,
    /// it is removed from the pipe.
    ///
    /// # Errors
    /// - `ReadError::BrokenPipe`: The pipe is empty and there are no writers,
    /// meaning that the pipe will never be written to again and the reader
    /// should stop reading.
    pub fn read_byte(&self) -> Result<u8, ReadError> {
        loop {
            if let Some(data) = self.pipe.buffer.lock().pop_front() {
                return Ok(data);
            }
            if self.pipe.writer_count() == 0 {
                return Err(ReadError::BrokenPipe);
            }
            self.pipe.waiting_writers.wake_up_someone();
            self.pipe.waiting_readers.sleep();
        }
    }

    /// Signal one writer to wake up. This is needed after reading from a pipe
    /// that was previously full in order to wake up a writer that is blocked
    /// on a full pipe.
    pub fn signal_one_writer(&self) {
        self.pipe.waiting_writers.wake_up_someone();
    }
}

impl Clone for PipeReader {
    fn clone(&self) -> Self {
        self.pipe.create_reader()
    }
}

impl Drop for PipeReader {
    /// When the reader is dropped, the number of readers is decremented and
    /// one writer is signaled to wake up, potentially unblocking it. This is
    /// needed if the writer is blocked on an empty pipe and the dropped reader
    /// was the last reader.
    fn drop(&mut self) {
        self.pipe.decrement_readers();
        self.pipe.waiting_writers.wake_up_someone();
    }
}

pub enum ReadError {
    /// The pipe is empty and there are no writers, meaning that the pipe will
    /// never be written to again and the reader should stop reading.
    BrokenPipe,
}
