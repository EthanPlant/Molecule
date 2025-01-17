use core::fmt::{self, Write};

use bitflags::bitflags;
use spin::{Mutex, Once};

use crate::arch::io;

pub static COM_1: Once<Mutex<SerialPort>> = Once::new();

#[derive(Debug)]
pub enum SerialError {
    SelfTestFailed,
}

bitflags! {
    #[derive(Copy, Clone)]
    struct LineStatus: u8 {
        const DATA_READY = 1 << 0;
        const OVERRUN_ERROR = 1 << 1;
        const PARITY_ERROR = 1 << 2;
        const FRAMING_ERROR = 1 << 3;
        const BREAK_INTERRUPT = 1 << 4;
        const TRANSMITTER_HOLDING_REGISTER_EMPTY = 1 << 5;
    }
}

type Result<T> = core::result::Result<T, SerialError>;

/// An interface for the 16550 UART serial port interface.
#[repr(transparent)]
pub struct SerialPort(u16);

impl SerialPort {
    /// Creates a new serial port interface with the specified port.
    /// 
    /// This function does not initialize the serial port. To initialize the serial port, call the
    /// [init] method.
    pub const fn new(port: u16) -> Self {
        Self(port)
    }

    /// Attempt to initialize the serial port, and returns the initialized port if successful.
    /// In order to ensure the serial port is functional, a self-test is performed. If the self
    /// 
    /// # Safety
    /// 
    /// The caller must ensure that the port is a serial port
    /// and that the port is valid as attempting to write to a non-existent port
    /// can lead to undefined behavior.
    pub unsafe fn init(self) -> Result<Self> {
        io::outb(self.0 + 1, 0x00); // Disable interrupts
        io::outb(self.0 + 3, 0x80); // Enable DLAB (set baud rate divisor)
        io::outb(self.0 + 0, 0x03); // Set divisor to 3 (lo byte) 38400 baud
        io::outb(self.0 + 1, 0x00); //                  (hi byte)
        io::outb(self.0 + 3, 0x03); // 8 bits, no parity, one stop bit
        io::outb(self.0 + 2, 0xC7); // Enable FIFO, clear them, with 14-byte threshold
        io::outb(self.0 + 4, 0x0B); // IRQs enabled, RTS/DSR set

        if !self.self_test() {
            return Err(SerialError::SelfTestFailed);
        }
        io::outb(self.0 + 4, 0x0F); // Set serial to normal operation mode
        Ok(self)
    }

    /// Write a byte to the serial port.
    pub fn write_byte(&self, byte: u8) {
        self.wait_for_status(LineStatus::TRANSMITTER_HOLDING_REGISTER_EMPTY);
        // Safety: The serial port is guaranteed to be initialized by the time this method is called
        // and the status is checked to ensure the transmitter holding register is empty.
        unsafe {
            io::outb(self.0, byte);
        }
    }

    pub fn read_byte(&mut self) -> u8 {
        // Safety: The serial port is guaranteed to be initialized by the time this method is called,
        // and the status is checked to ensure data is ready to be read.
        self.wait_for_status(LineStatus::DATA_READY);
        unsafe {
            io::inb(self.0)
        }
    }

    fn get_line_status(&self) -> LineStatus {
        LineStatus::from_bits_truncate(unsafe {io::inb(self.0 + 5)})
    }
    
    fn wait_for_status(&self, status: LineStatus) {
        while !self.get_line_status().contains(status) {}
    }

    unsafe fn self_test(&self) -> bool {
        io::outb(self.0 + 4, 0x1E); // Enable loopback mode
        io::outb(self.0 + 0, 0xAE); // Send test byte
        io::inb(self.0) == 0xAE
    }
}

/// Initialize the COM1 serial port if available.
pub fn init() {
    unsafe {
        let com_1 = SerialPort::new(0x3F8).init().unwrap();
        COM_1.call_once(|| Mutex::new(com_1));
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

pub macro serial_print($($arg:tt)*) {
    ($crate::drivers::uart_16650::_serial_print(format_args!($($arg)*)))
}

pub macro serial_println {
    () => ($crate::drivers::uart_16650::serial_print!("\n")),
    ($($arg:tt)*) => ($crate::drivers::uart_16650::serial_print!("{}\n", format_args!($($arg)*))),
}

#[doc(hidden)]
pub fn _serial_print(args: fmt::Arguments) {
    if let Some(c) = COM_1.get() {
        c.lock().write_fmt(args).expect("Failed to write to COM1");
    }
}