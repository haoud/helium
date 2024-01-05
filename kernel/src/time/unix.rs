#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnixTime(pub u64);

impl UnixTime {
    #[must_use]
    pub fn now() -> Self {
        // TODO: Get the current time from the RTC
        Self(0)
    }
}
