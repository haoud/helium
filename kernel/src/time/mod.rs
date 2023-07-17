use crate::x86_64::pit;
use core::{
    ops::{Add, AddAssign, Sub, SubAssign},
    sync::atomic::Ordering,
};

pub mod timer;

/// Represent a duration of time in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Second(pub u64);

impl Second {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl From<Millisecond> for Second {
    fn from(value: Millisecond) -> Self {
        Self(value.0 / 1000)
    }
}

impl From<Microsecond> for Second {
    fn from(value: Microsecond) -> Self {
        Self(value.0 / 1000000)
    }
}

impl From<Nanosecond> for Second {
    fn from(value: Nanosecond) -> Self {
        Self(value.0 / 1000000000)
    }
}

impl Add<Millisecond> for Second {
    type Output = Self;
    fn add(self, rhs: Millisecond) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub<Second> for Second {
    type Output = Self;
    fn sub(self, rhs: Second) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl AddAssign<Second> for Second {
    fn add_assign(&mut self, rhs: Second) {
        self.0 += rhs.0;
    }
}

impl SubAssign<Second> for Second {
    fn sub_assign(&mut self, rhs: Second) {
        self.0 -= rhs.0;
    }
}

/// Represent a duration of time in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Millisecond(pub u64);

impl Millisecond {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl From<Second> for Millisecond {
    fn from(value: Second) -> Self {
        Self(value.0 * 1000)
    }
}

impl From<Microsecond> for Millisecond {
    fn from(value: Microsecond) -> Self {
        Self(value.0 / 1000)
    }
}

impl From<Nanosecond> for Millisecond {
    fn from(value: Nanosecond) -> Self {
        Self(value.0 / 1000000)
    }
}

impl Add<Millisecond> for Millisecond {
    type Output = Self;
    fn add(self, rhs: Millisecond) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub<Millisecond> for Millisecond {
    type Output = Self;
    fn sub(self, rhs: Millisecond) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl AddAssign<Millisecond> for Millisecond {
    fn add_assign(&mut self, rhs: Millisecond) {
        self.0 += rhs.0;
    }
}

impl SubAssign<Millisecond> for Millisecond {
    fn sub_assign(&mut self, rhs: Millisecond) {
        self.0 -= rhs.0;
    }
}

/// Represent a duration of time in microseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Microsecond(pub u64);

impl Microsecond {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl From<Second> for Microsecond {
    fn from(value: Second) -> Self {
        Self(value.0 * 1000000)
    }
}

impl From<Millisecond> for Microsecond {
    fn from(value: Millisecond) -> Self {
        Self(value.0 * 1000)
    }
}

impl From<Nanosecond> for Microsecond {
    fn from(value: Nanosecond) -> Self {
        Self(value.0 / 1000)
    }
}

impl Add<Microsecond> for Microsecond {
    type Output = Self;
    fn add(self, rhs: Microsecond) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub<Microsecond> for Microsecond {
    type Output = Self;
    fn sub(self, rhs: Microsecond) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl AddAssign<Microsecond> for Microsecond {
    fn add_assign(&mut self, rhs: Microsecond) {
        self.0 += rhs.0;
    }
}

impl SubAssign<Microsecond> for Microsecond {
    fn sub_assign(&mut self, rhs: Microsecond) {
        self.0 -= rhs.0;
    }
}

/// Represent a duration of time in nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Nanosecond(pub u64);

impl Nanosecond {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

impl From<Second> for Nanosecond {
    fn from(value: Second) -> Self {
        Self(value.0 * 1000000000)
    }
}

impl From<Millisecond> for Nanosecond {
    fn from(value: Millisecond) -> Self {
        Self(value.0 * 1000000)
    }
}

impl From<Microsecond> for Nanosecond {
    fn from(value: Microsecond) -> Self {
        Self(value.0 * 1000)
    }
}

impl Add<Nanosecond> for Nanosecond {
    type Output = Self;
    fn add(self, rhs: Nanosecond) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub<Nanosecond> for Nanosecond {
    type Output = Self;
    fn sub(self, rhs: Nanosecond) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl AddAssign<Nanosecond> for Nanosecond {
    fn add_assign(&mut self, rhs: Nanosecond) {
        self.0 += rhs.0;
    }
}

impl SubAssign<Nanosecond> for Nanosecond {
    fn sub_assign(&mut self, rhs: Nanosecond) {
        self.0 -= rhs.0;
    }
}

/// Returns the uptime in nanoseconds.
#[must_use]
pub fn uptime_fast() -> Nanosecond {
    let tick = pit::TICK.load(Ordering::Relaxed);
    let ns = tick * 1000000000 / pit::FREQUENCY;
    Nanosecond::new(ns)
}

/// Returns the uptime in nanoseconds. This function is more precise than `uptime_fast` because
/// it compute the elapsed time since the last tick, but is much slower due to the additional
/// computation and/or I/O port access.
#[must_use]
pub fn uptime() -> Nanosecond {
    let tick = pit::TICK.load(Ordering::Relaxed);
    let ns = tick * 1000000000 / pit::FREQUENCY;
    let offset = pit::nano_offset();
    Nanosecond::new(ns + offset)
}
