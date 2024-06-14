pub struct Range {
    base: usize,
    len: usize,
}

impl Range {
    /// Creates a new range from a base and a length.
    ///
    /// # Panics
    /// Panics if the resulting range would overflow.
    #[must_use]
    pub fn new(base: usize, len: usize) -> Self {
        match Self::try_new(base, len) {
            None => panic!("Range overflow"),
            Some(range) => range,
        }
    }

    /// Tries to create a new range from a base and a length. Returns `None`
    /// if the resulting range would overflow.
    #[must_use]
    pub fn try_new(base: usize, len: usize) -> Option<Self> {
        base.checked_add(len).map(|_| Self { base, len })
    }

    /// Returns the base address of the range.
    #[must_use]
    pub fn base(&self) -> usize {
        self.base
    }

    /// Returns the length of the range.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Checks if the given range is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl From<core::ops::Range<usize>> for Range {
    fn from(range: core::ops::Range<usize>) -> Self {
        Self::new(range.start, range.end - range.start)
    }
}

impl core::fmt::Display for Range {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:016x} - 0x{:016x}[", self.base, self.base + self.len)
    }
}
