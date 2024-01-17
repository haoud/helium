use super::{units::Second, unix::UnixTime};
use crate::x86_64;

/// The date at which the kernel was started.
static STARTUP_DATE: Once<Date> = Once::new();

/// The Unix time at which the kernel was started.
static STARTUP_TIME: Once<UnixTime> = Once::new();

/// Number of days elased since the beginning of the year, excluding the current month.
const ELAPSED_DAYS_MONTHS: [usize; 12] = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Date {
    //// Converts the date to a Unix time. If the date is before January 1st, 1970,
    /// the Unix time returned will be 0.
    #[must_use]
    pub fn to_unix_time(&self) -> UnixTime {
        if self.year < 1970 {
            return UnixTime(Second(0));
        }

        let mut seconds = u64::from(self.second);
        seconds += u64::from(self.minute) * 60;
        seconds += u64::from(self.hour) * 60 * 60;
        seconds += (u64::from(self.day) - 1) * 60 * 60 * 24;
        seconds += ELAPSED_DAYS_MONTHS[self.month as usize - 1] as u64 * 60 * 60 * 24;
        seconds += (u64::from(self.year) - 1970) * 60 * 60 * 24 * 365;

        // Take into account leap years since 1970.
        seconds += (u64::from(self.year) - 1968) / 4 * 60 * 60 * 24;

        // If the current year is a leap year and the current month is January or February, we
        // need to remove one day from the total number of seconds.
        if self.year % 4 == 0 && self.month <= 2 {
            seconds -= 60 * 60 * 24;
        }

        UnixTime(Second(seconds))
    }
}

/// Initializes the startup date. The kernel assume that the date is configured
/// to the local time zone. However, most of the time, the date is configured to
/// the UTC time zone, and the kernel will display the wrong time.
///
/// Only Windows still configure the date to the local time zone by default for
/// backward compatibility reasons. I decided to use local time zone for simplicity
/// reasons, we are not the same (insert breaking bad meme here).
#[init]
pub fn setup() {
    STARTUP_DATE.call_once(read_slow);
    STARTUP_TIME.call_once(|| STARTUP_DATE.get().unwrap().to_unix_time());
}

/// Reads the date from the CMOS. This function is "slow" because it reads multiple
/// slow I/O ports in order to get the date.
#[must_use]
pub fn read_slow() -> Date {
    let years = 2000 + u16::from(x86_64::cmos::read(x86_64::cmos::Register::Year));

    Date {
        year: years,
        month: x86_64::cmos::read(x86_64::cmos::Register::Month),
        day: x86_64::cmos::read(x86_64::cmos::Register::Day),
        hour: x86_64::cmos::read(x86_64::cmos::Register::Hours),
        minute: x86_64::cmos::read(x86_64::cmos::Register::Minutes),
        second: x86_64::cmos::read(x86_64::cmos::Register::Seconds),
    }
}

/// Returns the date at which the kernel was started.
///
/// # Panics
/// Panics if the startup date has not been initialized. This
/// happens if the [`setup`] function has not been called before
/// calling this function.
#[must_use]
pub fn startup_date() -> Date {
    *STARTUP_DATE.get().expect("Startup date not initialized")
}

/// Returns the Unix time at which the kernel was started.
///
/// # Panics
/// Panics if the startup time has not been initialized. This
/// happens if the [`setup`] function has not been called before
/// calling this function.
pub fn startup_time() -> UnixTime {
    *STARTUP_TIME.get().expect("Startup time not initialized")
}
