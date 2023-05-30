use crate::io;

/// Represents a serial port. Currently, only COM1-4 are supported, and are statically mapped to
/// their respective addresses. This should be a safe assumption, as most x86_64 systems try to
/// keep compatibility with the original IBM PC. Furthermore, the serial ports are only used for
/// debugging purposes, and are not required for the kernel to function properly.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Port {
    COM1 = 0x3F8,
    COM2 = 0x2F8,
    COM3 = 0x3E8,
    COM4 = 0x2E8,
}

/// Represents a serial channel. This is used to interact with a serial port safely.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Serial {
    data: io::Port<u8>,
    interrupt_enable: io::Port<u8>,
    fifo_control: io::Port<u8>,
    line_control: io::Port<u8>,
    modem_control: io::Port<u8>,
    line_status: io::Port<u8>,
    _modem_status: io::Port<u8>,
    _scratch: io::Port<u8>,
}

impl Serial {
    /// Create a new serial channel and initialize the serial port. Currently, serial port are only
    /// used for debugging using QEMU's serial port, and this function even required to print
    /// anything to the QEMU console, so this function probably doesn't work on real hardware.
    #[must_use]
    pub fn new(com: Port) -> Serial {
        unsafe {
            let serial = Serial {
                data: io::Port::new(com as u16),
                interrupt_enable: io::Port::new(com as u16 + 1),
                fifo_control: io::Port::new(com as u16 + 2),
                line_control: io::Port::new(com as u16 + 3),
                modem_control: io::Port::new(com as u16 + 4),
                line_status: io::Port::new(com as u16 + 5),
                _modem_status: io::Port::new(com as u16 + 6),
                _scratch: io::Port::new(com as u16 + 7),
            };

            serial.interrupt_enable.write(0x00);
            serial.line_control.write(0x80);
            serial.data.write(0x03);
            serial.interrupt_enable.write(0x00);
            serial.line_control.write(0x03);
            serial.fifo_control.write(0xC7);
            serial.modem_control.write(0x0B);
            // We don't test if the line is ready to be written to here (I'm lazy)
            serial
        }
    }

    /// Check if the serial port is ready to be written to.
    #[must_use]
    pub fn is_transmit_empty(&self) -> bool {
        unsafe { self.line_status.read() & 0x20 != 0 }
    }

    /// Check if the serial port has data to be read.
    #[must_use]
    pub fn data_pending(&self) -> bool {
        unsafe { self.line_status.read() & 0x01 != 0 }
    }

    /// Write a byte to the serial port.
    pub fn write(&self, byte: u8) {
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }

        unsafe {
            self.data.write(byte);
        }
    }

    /// Read a byte from the serial port.
    #[must_use]
    pub fn read(&self) -> u8 {
        while !self.data_pending() {
            core::hint::spin_loop();
        }

        unsafe { self.data.read() }
    }
}

impl core::fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            self.write(byte);
        }
        Ok(())
    }
}
