use super::name::{InvalidName, Name};
use core::fmt::Display;

/// A path. The structure ensure that the path and its components are valid
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub components: Vec<Name>,
    pub absolute: bool,
}

impl Path {
    /// The maximum length of a path, in bytes. Since we use UTF-8, the maximum character
    /// count can be less than this.
    pub const MAX_LEN: usize = 4096;

    /// Creates a root path.
    #[must_use]
    pub const fn root() -> Self {
        Self {
            components: Vec::new(),
            absolute: true,
        }
    }

    /// Create a new path from a string.
    ///
    /// # Errors
    /// This function will return an error if the path is empty, contains a forbidden
    /// character (currently, the null byte), or is too long. See [`InvalidPath`] for more
    /// information about possible errors.
    pub fn new(path: &str) -> Result<Self, InvalidPath> {
        Self::validate(path)?;

        // The path is absolute if it starts with a forward slash.
        let absolute = path.chars().nth(0) == Some('/');

        // Split the path into components and validate each component to have a list
        // of valid names.
        let components = path
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| Name::new(String::from(s)))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            components,
            absolute,
        })
    }

    /// Validates a path to ensure it is valid.
    ///
    /// # Errors
    /// This function will return an error if the path is empty, contains a forbidden
    /// character (currently, the null byte), or is too long. See [`InvalidPath`] for more
    /// information about possible errors.
    pub fn validate(path: &str) -> Result<(), InvalidPath> {
        if path.is_empty() {
            return Err(InvalidPath::Empty);
        }
        if path.len() > Path::MAX_LEN {
            return Err(InvalidPath::TooLong);
        }
        if path.contains('\0') {
            return Err(InvalidPath::InvalidChar);
        }
        Ok(())
    }

    /// Iterate over the components of the path.
    pub fn iter(&self) -> impl Iterator<Item = &Name> {
        self.components.iter()
    }

    /// Push a new component to the path.
    pub fn push(&mut self, name: Name) {
        self.components.push(name);
    }

    /// Remove the last component of the path and return it.
    #[must_use]
    pub fn pop(&mut self) -> Option<Name> {
        self.components.pop()
    }

    /// Return true if the path is absolute (starts with a forward slash), false otherwise.
    #[must_use]
    pub const fn is_absolute(&self) -> bool {
        self.absolute
    }

    /// Return true if the path is relative (does not start with a forward slash), false
    /// otherwise.
    #[must_use]
    pub const fn is_relative(&self) -> bool {
        !self.absolute
    }

    /// Return the number of components in the path.
    #[must_use]
    pub fn count(&self) -> usize {
        self.components.len()
    }
}

impl From<Vec<Name>> for Path {
    fn from(components: Vec<Name>) -> Self {
        Self {
            components,
            absolute: false,
        }
    }
}

impl From<&[Name]> for Path {
    fn from(components: &[Name]) -> Self {
        Self {
            components: components.to_vec(),
            absolute: false,
        }
    }
}

impl IntoIterator for Path {
    type IntoIter = alloc::vec::IntoIter<Self::Item>;
    type Item = Name;

    fn into_iter(self) -> Self::IntoIter {
        self.components.into_iter()
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.absolute {
            write!(f, "/")?;
        }
        for (i, component) in self.components.iter().enumerate() {
            if i != 0 {
                write!(f, "/")?;
            }
            write!(f, "{}", component.as_str())?;
        }
        Ok(())
    }
}

/// Errors that can occur when validating a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InvalidPath {
    // An forbidden character was found in the path. Currently, the only forbidden character is
    // the null byte.
    InvalidChar,

    /// The path is not valid UTF-8.
    NotUTF8,

    /// An entry in the path is too long (more than [`Name::MAX_LEN`] bytes).
    NameTooLong,

    /// The path is too long (more than [`Path::MAX_LEN`] bytes).
    TooLong,

    /// The path is empty.
    Empty,
}

impl From<InvalidName> for InvalidPath {
    fn from(e: InvalidName) -> Self {
        match e {
            InvalidName::InvalidChar => InvalidPath::InvalidChar,
            InvalidName::TooLong => InvalidPath::NameTooLong,
            InvalidName::NotUTF8 => InvalidPath::NotUTF8,
            InvalidName::Empty => InvalidPath::Empty,
        }
    }
}
