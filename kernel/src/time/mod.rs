use self::units::Nanosecond;
use crate::x86_64::pit;
use core::sync::atomic::Ordering;

pub mod timer;
pub mod units;
pub mod unix;

/// Returns the uptime in nanoseconds.
#[must_use]
pub fn uptime_fast() -> Nanosecond {
    let tick = pit::TICK.load(Ordering::Relaxed);
    let ns = tick * 1000000000 / pit::FREQUENCY;
    Nanosecond::new(ns)
}

/// Returns the uptime in nanoseconds. This function is more precise than `uptime_fast` because
/// it compute the elapsed time since the last tick, but is much slower due to the additional
/// computation and/or extra I/O port access.
#[must_use]
pub fn uptime() -> Nanosecond {
    let tick = pit::TICK.load(Ordering::Relaxed);
    let ns = tick * 1000000000 / pit::FREQUENCY;
    let offset = pit::nano_offset();
    Nanosecond::new(ns + offset)
}
