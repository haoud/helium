use crate::x86_64::serial::{Port, Serial};
use core::fmt::Write;

pub static SERIAL: Lazy<Spinlock<Serial>> =
    Lazy::new(|| Spinlock::new(unsafe { Serial::new(Port::COM1) }));

struct Logger;

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let level = match record.level() {
                log::Level::Error => "\x1b[1m\x1b[31m[!]\x1b[0m",
                log::Level::Warn => "\x1b[1m\x1b[33m[-]\x1b[0m",
                log::Level::Info => "\x1b[1m\x1b[32m[*]\x1b[0m",
                log::Level::Debug => "\x1b[1m\x1b[34m[#]\x1b[0m",
                log::Level::Trace => "\x1b[1m[~]\x1b[0m",
            };

            // Write the log message to the serial port, ignoring any error
            _ = SERIAL
                .lock()
                .write_fmt(format_args!("{} {}\n", level, record.args()));
        }
    }

    fn flush(&self) {}
}

/// Initialize the logger. This function should be called before any other logging function.
/// Currently, this function initialize the serial port and set it as the logger.
///
/// # Panics
/// Panics if the logger is already set.
#[init]
pub fn setup() {
    log::set_logger(&Logger).expect("A logger is already set");
    log::set_max_level(log::LevelFilter::Error);
}

/// Called when the kernel panics. This function force the unlock of the serial port, because the
/// panic handle could be called while the serial port is locked, which would cause a deadlock and
/// prevent the panic message from being printed
///
/// # Safety
/// This function is unsafe because it force the unlock of the serial port, which could cause a
/// undefined behavior if the serial port is used by several threads after this function call.
/// This is the caller responsability to ensure that thelogging system will only be used by one
/// thread after this function call.
#[cold]
pub unsafe fn on_panic() {
    SERIAL.force_unlock();
}
