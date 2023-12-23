use core::ops::{Add, AddAssign, Sub, SubAssign};

/// Represent a duration of time in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Second(pub u64);

impl Second {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_mul(self, rhs: u64) -> Option<Self> {
        self.0.checked_mul(rhs).map(Self)
    }

    #[must_use]
    pub fn checked_div(self, rhs: u64) -> Option<Self> {
        self.0.checked_div(rhs).map(Self)
    }

    #[must_use]
    pub fn saturated_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    #[must_use]
    pub fn saturated_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    #[must_use]
    pub fn saturated_mul(self, rhs: u64) -> Self {
        Self(self.0.saturating_mul(rhs))
    }

    #[must_use]
    pub fn saturated_div(self, rhs: u64) -> Self {
        Self(self.0.saturating_div(rhs))
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

impl Add<Second> for Second {
    type Output = Self;
    fn add(self, rhs: Second) -> Self::Output {
        self.checked_add(rhs).expect("Overflow when adding seconds")
    }
}

impl Sub<Second> for Second {
    type Output = Self;
    fn sub(self, rhs: Second) -> Self::Output {
        self.checked_sub(rhs)
            .expect("Overflow when substracting seconds")
    }
}

impl AddAssign<Second> for Second {
    fn add_assign(&mut self, rhs: Second) {
        *self = *self + rhs;
    }
}

impl SubAssign<Second> for Second {
    fn sub_assign(&mut self, rhs: Second) {
        *self = *self - rhs;
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

    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_mul(self, rhs: u64) -> Option<Self> {
        self.0.checked_mul(rhs).map(Self)
    }

    #[must_use]
    pub fn checked_div(self, rhs: u64) -> Option<Self> {
        self.0.checked_div(rhs).map(Self)
    }

    #[must_use]
    pub fn saturated_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    #[must_use]
    pub fn saturated_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    #[must_use]
    pub fn saturated_mul(self, rhs: u64) -> Self {
        Self(self.0.saturating_mul(rhs))
    }

    #[must_use]
    pub fn saturated_div(self, rhs: u64) -> Self {
        Self(self.0.saturating_div(rhs))
    }
}

impl From<Second> for Millisecond {
    fn from(value: Second) -> Self {
        Self(
            value
                .0
                .checked_mul(1000)
                .expect("Overflow when converting seconds to milliseconds"),
        )
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
        self.checked_add(rhs)
            .expect("Overflow when adding milliseconds")
    }
}

impl Sub<Millisecond> for Millisecond {
    type Output = Self;
    fn sub(self, rhs: Millisecond) -> Self::Output {
        self.checked_sub(rhs)
            .expect("Overflow when substracting milliseconds")
    }
}

impl AddAssign<Millisecond> for Millisecond {
    fn add_assign(&mut self, rhs: Millisecond) {
        *self = *self + rhs;
    }
}

impl SubAssign<Millisecond> for Millisecond {
    fn sub_assign(&mut self, rhs: Millisecond) {
        *self = *self - rhs;
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

    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_mul(self, rhs: u64) -> Option<Self> {
        self.0.checked_mul(rhs).map(Self)
    }

    #[must_use]
    pub fn checked_div(self, rhs: u64) -> Option<Self> {
        self.0.checked_div(rhs).map(Self)
    }

    #[must_use]
    pub fn saturated_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    #[must_use]
    pub fn saturated_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    #[must_use]
    pub fn saturated_mul(self, rhs: u64) -> Self {
        Self(self.0.saturating_mul(rhs))
    }

    #[must_use]
    pub fn saturated_div(self, rhs: u64) -> Self {
        Self(self.0.saturating_div(rhs))
    }
}

impl From<Second> for Microsecond {
    fn from(value: Second) -> Self {
        Self(
            value
                .0
                .checked_mul(1000000)
                .expect("Overflow when converting seconds to microseconds"),
        )
    }
}

impl From<Millisecond> for Microsecond {
    fn from(value: Millisecond) -> Self {
        Self(
            value
                .0
                .checked_mul(1000)
                .expect("Overflow when converting milliseconds to microseconds"),
        )
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
        self.checked_add(rhs)
            .expect("Overflow when adding microseconds")
    }
}

impl Sub<Microsecond> for Microsecond {
    type Output = Self;
    fn sub(self, rhs: Microsecond) -> Self::Output {
        self.checked_sub(rhs)
            .expect("Overflow when substracting microseconds")
    }
}

impl AddAssign<Microsecond> for Microsecond {
    fn add_assign(&mut self, rhs: Microsecond) {
        *self = *self + rhs;
    }
}

impl SubAssign<Microsecond> for Microsecond {
    fn sub_assign(&mut self, rhs: Microsecond) {
        *self = *self - rhs;
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

    #[must_use]
    pub const fn zero() -> Self {
        Self(0)
    }

    #[must_use]
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }

    #[must_use]
    pub fn checked_mul(self, rhs: u64) -> Option<Self> {
        self.0.checked_mul(rhs).map(Self)
    }

    #[must_use]
    pub fn checked_div(self, rhs: u64) -> Option<Self> {
        self.0.checked_div(rhs).map(Self)
    }

    #[must_use]
    pub fn saturated_add(self, rhs: Self) -> Self {
        Self(self.0.saturating_add(rhs.0))
    }

    #[must_use]
    pub fn saturated_sub(self, rhs: Self) -> Self {
        Self(self.0.saturating_sub(rhs.0))
    }

    #[must_use]
    pub fn saturated_mul(self, rhs: u64) -> Self {
        Self(self.0.saturating_mul(rhs))
    }

    #[must_use]
    pub fn saturated_div(self, rhs: u64) -> Self {
        Self(self.0.saturating_div(rhs))
    }
}

impl From<Second> for Nanosecond {
    fn from(value: Second) -> Self {
        Self(
            value
                .0
                .checked_mul(1000000000)
                .expect("Overflow when converting seconds to nanoseconds"),
        )
    }
}

impl From<Millisecond> for Nanosecond {
    fn from(value: Millisecond) -> Self {
        Self(
            value
                .0
                .checked_mul(1000000)
                .expect("Overflow when converting milliseconds to nanoseconds"),
        )
    }
}

impl From<Microsecond> for Nanosecond {
    fn from(value: Microsecond) -> Self {
        Self(
            value
                .0
                .checked_mul(1000)
                .expect("Overflow when converting microseconds to nanoseconds"),
        )
    }
}

impl Add<Nanosecond> for Nanosecond {
    type Output = Self;
    fn add(self, rhs: Nanosecond) -> Self::Output {
        self.checked_add(rhs)
            .expect("Overflow when adding nanoseconds")
    }
}

impl Sub<Nanosecond> for Nanosecond {
    type Output = Self;
    fn sub(self, rhs: Nanosecond) -> Self::Output {
        self.checked_sub(rhs)
            .expect("Overflow when substracting nanoseconds")
    }
}

impl AddAssign<Nanosecond> for Nanosecond {
    fn add_assign(&mut self, rhs: Nanosecond) {
        *self = *self + rhs;
    }
}

impl SubAssign<Nanosecond> for Nanosecond {
    fn sub_assign(&mut self, rhs: Nanosecond) {
        *self = *self - rhs;
    }
}
