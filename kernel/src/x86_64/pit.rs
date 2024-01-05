use super::io::Port;
use core::sync::atomic::{AtomicU64, Ordering};

/// The number of tick elapsed since the boot of the kernel. This variable is incremented every
/// time the PIT generates an IRQ.
pub static TICK: AtomicU64 = AtomicU64::new(0);

static CHANNEL_0: Port<u8> = Port::new(0x40);
static CHANNEL_1: Port<u8> = Port::new(0x41);
static CHANNEL_2: Port<u8> = Port::new(0x42);
static COMMAND: Port<u8> = Port::new(0x43);

/// The number of nanoseconds between each PIT internal tick.
pub const PIT_TICK_NS: u64 = 1_000_000_000 / 1_193_180;

/// The internal frequency of the PIT, in Hz. This is the frequency of the internal clock that
/// drives the PIT, and is not the frequency that the PIT can be set to.
pub const PIT_FREQ: u64 = 1_193_180;

/// The frequency of the desired PIT frequency, in Hz.
pub const FREQUENCY: u64 = 200;

/// The value to load into the PIT to get the desired frequency.
pub const LATCH: u64 = PIT_FREQ / FREQUENCY;

/// Setup the PIT to generate IRQ0 at the desired frequency
///
/// # Safety
/// This function is unsafe because it use I/O ports to communicate with the PIT, and might cause
/// undefined behavior or memory safety issues if used incorrectly.
#[init]
pub unsafe fn setup() {
    let high = ((LATCH >> 8) & 0xFF) as u8;
    let low = (LATCH & 0xFF) as u8;

    // Set channel 0 to mode 3 (square wave generator), binary format
    COMMAND.write(0x36);

    // Set the frequency divisor
    CHANNEL_0.write(low);
    CHANNEL_0.write(high);
}

/// Returns the elapsed time since the last IRQ in nanoseconds. In order to do that, it reads the
/// current value of the counter and calculates the elapsed time since the last IRQ. Since this
/// function read through the PIT and I/O ports, it is not very fast, and should not be called
/// often.
#[must_use]
pub fn nano_offset() -> u64 {
    // Read the current value of the counter (channel 0)
    let counter = unsafe {
        COMMAND.write(0);
        let low = u64::from(CHANNEL_0.read());
        let high = u64::from(CHANNEL_0.read());
        (high << 8) | low
    };

    // Calculate the elapsed time since the last IRQ
    let elapsed = LATCH - counter;
    elapsed * PIT_TICK_NS
}

/// Called every time the PIT generates an IRQ.
pub fn timer_tick() {
    TICK.fetch_add(1, Ordering::Relaxed);
}
