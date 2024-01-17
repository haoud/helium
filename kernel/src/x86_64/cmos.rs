use super::{io::Port, irq};

static IO_ADDRESS: Port<u8> = Port::new(0x70);
static IO_DATA: Port<u8> = Port::new(0x71);

/// Represents a CMOS register. Each register is 1 byte wide and contains a binary-coded
/// decimal (BCD) value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Register {
    Seconds = 0x00,
    Minutes = 0x02,
    Hours = 0x04,
    Day = 0x07,
    Month = 0x08,
    Year = 0x09,
}

/// Converts a binary-coded decimal (BCD) value to a binary value.
const fn bcd2bin(value: u8) -> u8 {
    (value >> 4) * 10 + (value & 0x0F)
}

/// Converts a binary value to a binary-coded decimal (BCD) value.
const fn bin2bcd(value: u8) -> u8 {
    ((value / 10) << 4) + (value % 10)
}

/// Returns the value of the given CMOS register. The value is converted from
/// binary-coded decimal (BCD) to binary.
#[must_use]
pub fn read(reg: Register) -> u8 {
    irq::without(|| unsafe {
        IO_ADDRESS.write(mni_bit() | reg as u8);
        bcd2bin(IO_DATA.read())
    })
}

/// Writes the given value to the given CMOS register.
pub fn write(_reg: Register, _value: u8) {
    unimplemented!("Write to CMOS may be dangerous and is not implemented");
}

#[must_use]
fn mni_bit() -> u8 {
    unsafe { IO_ADDRESS.read() & 0x80 }
}
