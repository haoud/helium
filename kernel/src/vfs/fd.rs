use super::file::OpenFile;

/// A file descriptor. This is an identifier for an opened file, unique to the
/// process. Currently, it is just an index in the table of opened files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Descriptor(pub usize);

/// A table of opened files. This is a simple array of 32 elements, where each
/// element is an optional reference to an opened file.
///
/// The descriptor of a file is its index in this table.
#[derive(Default, Debug, Clone)]
pub struct OpenedFiles {
    files: [Option<Arc<OpenFile>>; 32],
}

impl OpenedFiles {
    /// Create a new empty table of opened files.
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Insert the given file into the table. Returns the descriptor of the
    /// inserted file if there is space left, `None` otherwise.
    #[must_use]
    pub fn insert(&mut self, file: Arc<OpenFile>) -> Option<Descriptor> {
        for (i, slot) in self.files.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(file);
                return Some(Descriptor(i));
            }
        }
        None
    }

    /// Remove the file corresponding to the given descriptor. Returns the file
    /// if the descriptor is valid, `None` otherwise.
    pub fn remove(&mut self, fd: Descriptor) -> Option<Arc<OpenFile>> {
        self.files[fd.0].take()
    }

    /// Get the file corresponding to the given descriptor. Returns the file if
    /// the descriptor is valid, `None` otherwise.
    #[must_use]
    pub fn get(&self, fd: Descriptor) -> Option<&Arc<OpenFile>> {
        self.files[fd.0].as_ref()
    }
}
