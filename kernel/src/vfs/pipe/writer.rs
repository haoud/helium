use super::Pipe;

/// A pipe writer. This is a wrapper around a pipe that provides a safe
/// interface for writing to the pipe.
pub struct PipeWriter {
    pipe: Arc<Pipe>,
}

impl PipeWriter {
    #[must_use]
    pub fn new(pipe: Arc<Pipe>) -> Self {
        Self { pipe }
    }

    /// Writes a byte to the pipe. If the pipe is full, the current thread will
    /// be put to sleep until a byte is removed from the pipe.
    ///
    /// # Errors
    /// - `WriteError::BrokenPipe`: The pipe is full and there are no readers,
    ///     meaning that the pipe will never be read from again and the writer
    ///     should stop writing.
    pub fn write_byte(&self, data: u8) -> Result<(), WriteError> {
        while self.pipe.buffer.lock().try_push_back(data).is_err() {
            if self.pipe.reader_count() == 0 {
                return Err(WriteError::BrokenPipe);
            }

            self.pipe.waiting_readers.wake_up_someone();
            self.pipe.waiting_writers.sleep();
        }
        Ok(())
    }

    /// Signal one reader to wake up. This is needed after writing to a pipe
    /// that was previously empty in order to wake up a reader that is blocked
    /// on an empty pipe.
    pub fn signal_one_reader(&self) {
        self.pipe.waiting_readers.wake_up_someone();
    }
}

impl Clone for PipeWriter {
    fn clone(&self) -> Self {
        self.pipe.create_writer()
    }
}

impl Drop for PipeWriter {
    /// When the writer is dropped, the number of writers is decremented and
    /// one reader is signaled to wake up, potentially unblocking it. This is
    /// needed if the reader is blocked on a full pipe and the dropped writer
    /// was the last writer.
    fn drop(&mut self) {
        self.pipe.decrement_writers();
        self.pipe.waiting_readers.wake_up_someone();
    }
}

pub enum WriteError {
    /// The pipe is full and there are no readers, meaning that the pipe will
    /// never be read from again and the writer should stop writing.
    BrokenPipe,
}
