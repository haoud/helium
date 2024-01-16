use super::{units::Second, date::{self, Date}, uptime_fast};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnixTime(pub Second);

impl UnixTime {
    /// Returns the current Unix time.
    #[must_use]
    pub fn now() -> Self {
        let startup = date::startup_time();
        let uptime = Second::from(uptime_fast());
        UnixTime(startup.0 + uptime)
    }
}

impl From<Date> for UnixTime {
    fn from(date: Date) -> Self {
        date.to_unix_time()
    }
}

impl From<UnixTime> for u64 {
    fn from(val: UnixTime) -> Self {
        val.0.0
    }
}