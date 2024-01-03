/// An device identifier. It is composed of a 32 bits major number and a 32 bits minor number.
/// The major number identifies the type of the device (for example, a disk driver) and the minor
/// number identifies the specific device (for example, the first disk is 0, the second is 1, etc).
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Identifier {
    pub major: u32,
    pub minor: u32,
}

/// Represents a device type as well as its identifier. This is useful to have all the information
/// about a device, because a block device and a char device can have the same identifier, but two
/// block devices or two char devices cannot have the same identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Device {
    Block(Identifier),
    Char(Identifier),
    None,
}

impl Device {
    /// Returns the major number of the device.
    #[must_use]
    pub fn major(&self) -> u32 {
        match *self {
            Device::Char(id) | Device::Block(id) => id.major,
            Device::None => 0,
        }
    }

    /// Returns the minor number of the device.
    #[must_use]
    pub fn minor(&self) -> u32 {
        match *self {
            Device::Char(id) | Device::Block(id) => id.minor,
            Device::None => 0,
        }
    }

    /// Returns the device identifier as a 64-bit integer. A block device and char device can have
    /// the same identifier, but two block devices or two char devices cannot have the same
    /// identifier.
    #[must_use]
    #[allow(clippy::cast_lossless)]
    pub fn id(&self) -> u64 {
        match *self {
            Device::Char(id) | Device::Block(id) => (id.major as u64) << 32 | (id.minor as u64),
            Device::None => 0,
        }
    }

    /// Return the next device identifier. It will share the same major number, but the minor
    /// number will be incremented by one.
    ///
    /// # Panics
    /// Panic if an overflow occurs when incrementing the minor number, or if the device identifier
    /// is `None`.
    #[must_use]
    pub fn next(&self) -> Self {
        match *self {
            Device::Char(id) => Device::Char(Identifier {
                major: id.major,
                minor: id.minor + 1,
            }),
            Device::Block(id) => Device::Block(Identifier {
                major: id.major,
                minor: id.minor + 1,
            }),
            Device::None => panic!("Cannot increment the identifier of a None device"),
        }
    }
}
