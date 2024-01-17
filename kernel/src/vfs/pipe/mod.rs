use self::{reader::PipeReader, writer::PipeWriter};
use super::file;
use crate::user::task::queue::WaitQueue;
use circular_buffer::CircularBuffer;
use core::sync::atomic::{AtomicUsize, Ordering};

static PIPE_FILE_OPS: file::FileOperation = file::FileOperation { write, read, seek };

pub mod reader;
pub mod writer;

/// A pipe is a mechanism for interprocess communication (IPC) that allows
/// the transfer of data between two processes. A pipe has two ends: a read
/// end and a write end. Data written to the write end of the pipe can be
/// read from the read end of the pipe. A pipe is a first-in-first-out (FIFO)
/// data structure, meaning that the first byte written to the pipe is the
/// first byte that can be read from the pipe.
/// 
/// A pipe is implemented as a circular buffer that stores the data written
/// to the pipe. The read end of the pipe reads data from the buffer and the
/// write end of the pipe writes data to the buffer. If the buffer is full,
/// the write end of the pipe will block until there is space in the buffer.
/// If the buffer is empty, the read end of the pipe will block until there
/// is data in the buffer.
/// 
/// Writing or reading from a pipe is atomic only if the size of the data
/// being written or read is less than or equal to [`Pipe::BUFFER_SIZE`],
/// and if the data can be written or read in a single operation. If the
/// data is larger than [`Pipe::BUFFER_SIZE`] or if the data cannot be
/// written or read in a single operation, the operation is not atomic.
pub struct Pipe {
    /// The buffer that stores the data in the pipe. 
    buffer: Mutex<Box<CircularBuffer<{Pipe::BUFFER_SIZE}, u8>>>,

    /// A list of readers that are blocked on an empty pipe.
    waiting_readers: WaitQueue,

    /// A list of writers that are blocked on a full pipe.
    waiting_writers: WaitQueue,

    /// The number of readers. If the number of readers is zero, the pipe
    /// is broken and the writer should stop writing.
    readers: AtomicUsize,

    /// The number of writers. If the number of writers is zero, the pipe
    /// is broken and the reader should stop reading.
    writers: AtomicUsize,
}

impl Pipe {
    /// The size of the pipe buffer in bytes. This is the maximum number of bytes
    /// that can be stored in the pipe at any given time.
    pub const BUFFER_SIZE: usize = 4096;

    /// Creates a new pipe.
    #[must_use]
    pub fn new() -> Arc<Pipe> {
        Arc::new(Pipe {
            buffer: Mutex::new(CircularBuffer::boxed()),
            waiting_readers: WaitQueue::new(),
            waiting_writers: WaitQueue::new(),
            readers: AtomicUsize::new(0),
            writers: AtomicUsize::new(0),
        })
    }

    /// Creates a new pipe reader and increments the number of readers.
    /// When the reader is dropped, the number of readers is automatically
    /// decremented by the destructor.
    #[must_use]
    pub fn create_reader(self: &Arc<Pipe>) -> PipeReader {
        self.increment_readers();
        PipeReader::new(Arc::clone(self))
    }

    /// Creates a new pipe writer and increments the number of writers.
    /// When the writer is dropped, the number of writers is automatically
    /// decremented by the destructor.
    #[must_use]
    pub fn create_writer(self: &Arc<Pipe>) -> PipeWriter {
        self.increment_writers();
        PipeWriter::new(Arc::clone(self))
    }

    /// Increments the number of readers and returns the new value.
    fn increment_readers(&self) -> usize {
        self.readers.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrements the number of readers and returns the new value.
    fn decrement_readers(&self) -> usize {
        self.readers.fetch_sub(1, Ordering::SeqCst) - 1
    }

    /// Increments the number of writers and returns the new value.
    fn increment_writers(&self) -> usize {
        self.writers.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrements the number of writers and returns the new value.
    fn decrement_writers(&self) -> usize {
        self.writers.fetch_sub(1, Ordering::SeqCst) - 1
    }

    /// Returns the number of readers.
    fn reader_count(&self) -> usize {
        self.readers.load(Ordering::SeqCst)
    }

    /// Returns the number of writers.
    fn writer_count(&self) -> usize {
        self.writers.load(Ordering::SeqCst)
    }
}

/// Creates a new pipe and returns a pair of files, the first one for reading
/// from the pipe and the second one for writing to the pipe.
fn create_pair() -> (Arc<file::File>, Arc<file::File>) {
    let pipe = Pipe::new();
    let reader = pipe.create_reader();
    let writer = pipe.create_writer();

    let reader_file = Arc::new(file::File::new(file::FileCreateInfo {
        operation: file::Operation::File(&PIPE_FILE_OPS),
        open_flags: file::OpenFlags::READ,
        data: Box::new(reader),
        inode: None,
    }));

    let writer_file = Arc::new(file::File::new(file::FileCreateInfo {
        operation: file::Operation::File(&PIPE_FILE_OPS),
        open_flags: file::OpenFlags::WRITE,
        data: Box::new(writer),
        inode: None,
    }));

    (reader_file, writer_file)
}

/// Writes data to a pipe. If the pipe is full, the current thread will be put
/// to sleep until there is space in the pipe.
/// 
/// Since a pipe behaves like a character device, the offset is ignored.
/// 
/// # Errors
/// - `WriteError::BrokenPipe`: The pipe is full and there are no readers,
///  meaning that the pipe will never be read from again and the writer should
/// stop writing.
/// 
/// Partial writes can occur if the pipe is broken, but some data was written
/// before the pipe was permanently full.
/// 
/// # Panics
/// Panics if the file is not a pipe writer.
fn write(file: &file::File, buf: &[u8], _offset: file::Offset) -> Result<usize, file::WriteError> {
    let pipe_writer = file
        .data
        .downcast_ref::<PipeWriter>()
        .expect("Trying to write into file that is not a pipe");

    let mut written = 0;
    for &byte in buf {
        match pipe_writer.write_byte(byte) {
            // The pipe is full and there are no readers, meaning that the pipe
            // will never be read from again and the writer should stop writing.
            // However, if the writer has already written some bytes, we return
            // the number of bytes written. The broken pipe error will be
            // returned on the next write.
            Err(writer::WriteError::BrokenPipe) if written == 0 => {
                return Err(file::WriteError::BrokenPipe)
            }
            Err(writer::WriteError::BrokenPipe) => break,
            Ok(()) => written += 1,
        }
    }

    // Signal one reader to wake up since there is new data in the pipe.
    pipe_writer.signal_one_reader();
    Ok(written)
}

/// Reads data from a pipe. If the pipe is empty, the current thread will be
/// put to sleep until there is data in the pipe. Once the data is read, it
/// is removed from the pipe.
/// 
/// Since a pipe behaves like a character device, the offset is ignored.
/// 
/// # Errors
/// - `ReadError::BrokenPipe`: The pipe is empty and there are no writers,
/// meaning that the pipe will never be written to again and the reader should
/// stop reading.
/// 
/// Partial reads can occur if the pipe is broken, but some data was read
/// before the pipe was permanently empty.
/// 
/// # Panics.
/// Panics if the file is not a pipe reader
fn read(
    file: &file::File,
    buf: &mut [u8],
    _offset: file::Offset,
) -> Result<usize, file::ReadError> {
    let pipe_reader = file
        .data
        .downcast_ref::<PipeReader>()
        .expect("Trying to read from file that is not a pipe");

    let mut readed = 0;
    for byte in buf {
        match pipe_reader.read_byte() {
            // The pipe is empty and there are no writers, meaning that the pipe
            // will never be written to again and the reader should stop reading.
            // However, if the reader has already read some bytes, we return the
            // number of bytes read. The broken pipe error will be returned on
            // the next read.
            Err(reader::ReadError::BrokenPipe) if readed == 0 => {
                return Err(file::ReadError::BrokenPipe)
            }
            Err(reader::ReadError::BrokenPipe) => break,
            Ok(data) => {
                *byte = data;
                readed += 1;
            }
        }
    }

    // Signal one writer to wake up since there is new space in the pipe.
    pipe_reader.signal_one_writer();
    Ok(readed)
}

/// Seeking in a pipe is not supported because it does not make sense since
/// the data in the pipe is not stored in a device but dynamically generated
/// and consumed when the data is read. Therefore, the seek operation is
/// always rejected.
fn seek(
    _file: &file::File,
    _offset: isize,
    _whence: file::Whence,
) -> Result<file::Offset, file::SeekError> {
    Err(file::SeekError::NotSeekable)
}
