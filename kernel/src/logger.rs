use cfg_if::cfg_if;
use core::fmt::Write;
use macros::init;
use sync::{Lazy, Spinlock};
use x86_64::serial::{Port, Serial};

static SERIAL: Lazy<Spinlock<Serial>> = Lazy::new(|| Spinlock::new(Serial::new(Port::COM1)));

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

            SERIAL
                .lock()
                .write_fmt(format_args!("{} {}\n", level, record.args()))
                .unwrap();
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
    cfg_if!(
        if #[cfg(feature = "log")] {
            let _ = log::set_logger(&Logger);
            log::set_max_level(log::LevelFilter::Trace);
        }
    );
}

/// Called when the kernel panics. This function force the unlock of the serial port, because the
/// panic handle could be called while the serial port is locked, which would cause a deadlock and
/// prevent the panic message from being printed
#[cold]
pub fn on_panic() {
    unsafe {
        SERIAL.force_unlock();
    }
}
