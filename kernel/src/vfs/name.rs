use alloc::string::String;

/// A name. The structure ensure that the name is valid.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(String);

impl Name {
    const MAX_LEN: usize = 255;

    /// Creates a new name from a string.
    ///
    /// # Errors
    /// This function will return an error if the name is empty, contains a forbidden
    /// character (currently, the null byte and the forward slash), or is too long
    /// (more than [`Name::MAX_LEN`] bytes).
    pub fn new(name: String) -> Result<Self, InvalidName> {
        Self::validate(&name)?;
        Ok(Self(name))
    }

    /// Validates a name.
    ///
    /// # Errors
    /// This function will return an error if the name is empty, contains a forbidden
    /// character (currently, the null byte and the forward slash), or is too long
    /// (more than [`Name::MAX_LEN`] bytes).
    pub fn validate(name: &str) -> Result<(), InvalidName> {
        if name.is_empty() {
            return Err(InvalidName::Empty);
        }
        if name.len() > Name::MAX_LEN {
            return Err(InvalidName::TooLong);
        }
        if name.contains('\0') || name.contains('/') {
            return Err(InvalidName::InvalidChar);
        }
        Ok(())
    }

    /// Returns the name as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Errors that can occur when validating a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InvalidName {
    // An forbidden character was found in the name. Currently, forbidden characters are the
    // null byte and the forward slash ('/').
    InvalidChar,

    /// The name is not valid UTF-8.
    NotUTF8,

    /// The name is too long (more than [`Name::MAX_LEN`] bytes).
    TooLong,

    /// The name is empty.
    Empty,
}
