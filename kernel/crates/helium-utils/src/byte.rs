/// A wrapper around a usize that represents a size in bytes.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteSize(pub usize);

impl ByteSize {
    pub const UNITS: &[&'static str] = &["B", "KiB", "MiB", "GiB", "TiB"];

    #[must_use]
    pub const fn new(size: usize) -> Self {
        Self(size)
    }
}

impl core::fmt::Display for ByteSize {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let mut value = self.0;
        let mut i = 0;
        while value >= 1024 && i < ByteSize::UNITS.len() - 1 {
            value /= 1024;
            i += 1;
        }
        write!(f, "{} {}", value, ByteSize::UNITS[i])
    }
}
