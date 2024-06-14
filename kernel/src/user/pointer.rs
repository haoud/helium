use addr::user::UserVirtual;

/// This structure encapsulate a pointer to an object in the userland memory:
/// this structure guarantees that the pointer is in the userland memory.
///
/// # Data Races
/// Contrary to the kernel, data races are allowed in the userland memory. This
/// is because multiple tasks can share the same memory space in the userland
/// memory, and therefore can pass at the same time the same pointer to the
/// kernel. This is the userland programmer responsibility to ensure that there
/// is no data races in their program: the kernel cannot ensure this because
/// not all user applications are written in Rust and follow the Rust memory
/// safety rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer<T> {
    inner: *mut T,
}

impl<T> Pointer<T> {
    /// Tries to create a new user pointer. Returns `None` if the given pointer
    /// is not in user space.
    pub fn new(ptr: *mut T) -> Option<Self> {
        if UserVirtual::is_user_ptr(ptr) {
            Some(Self { inner: ptr })
        } else {
            None
        }
    }

    /// Tries to create a new user pointer from a usize. Returns `None` if the
    /// resulting pointer would not be in user space.
    #[must_use]
    pub fn from_usize(ptr: usize) -> Option<Self> {
        Self::new(ptr as *mut T)
    }

    /// Tries to create a new user pointer from a u64. Returns `None` if the
    /// resulting pointer would not be in user space.
    #[must_use]
    pub fn from_u64(ptr: u64) -> Option<Self> {
        Self::new(ptr as *mut T)
    }

    /// Get the pointer to the object in the userland memory.
    #[must_use]
    pub const fn inner(&self) -> *mut T {
        self.inner
    }
}

impl<T> core::fmt::Display for Pointer<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:016x}", self.inner as u64)
    }
}

/// Represents an error that can occur when converting a raw
/// pointer to a user pointer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerError {
    /// The given pointer is not in the userland memory.
    NotInUserland,
}
