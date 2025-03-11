//! Support for the UART 16650 serial port.

use core::fmt::{self, Write};
use core::marker::PhantomData;

use spin::Once;

use crate::arch::io;
use crate::sync::Mutex;

/// Global access to the COM 1 port
static COM_1: Once<Mutex<SerialPort<Initialized>>> = Once::new();

const TRANSMIT_RECIEVE: u8 = 0;
const INTERRUPT_ENABLED: u8 = 1;
const BAUD_RATE_LSB: u8 = 0;
const BAUD_RATE_MSB: u8 = 1;
const FIFO_CONTROL: u8 = 2;
const LINE_CONTROL: u8 = 3;
const MODEM_CONTROL: u8 = 4;
const LINE_STATUS: u8 = 5;

const COM_1_ADDR: u16 = 0x3f8;

bitflags::bitflags! {
    /// Serial port status flags.
    #[derive(Copy, Clone)]
    struct LineStatus: u8 {
        /// There's data available to read.
        const DATA_READY = 1;
        /// Data has been lost.
        const OVERRUN_ERROR = 1 << 1;
        /// The parity bit detected an error in the transmission
        const PARITY_ERROR = 1 << 2;
        /// A stop bit was missing.
        const FRAMING_ERROR = 1 << 3;
        /// A break in data input.
        const BREAK = 1 << 4;
        /// The transmission buffer is empty (data can be sent).
        const TRANSMITTING_HOLDER_REGISTER_EMPTY = 1 << 5;
        /// The transmitter isn't doing anything.
        const TRANSMITTER_ERROR = 1 << 6;
        /// An error with a word in the input buffer.
        const IMPENDING_ERROR = 1 << 7;
    }
}

/// Marker trait for the serial port's status
trait SerialStatus {}

/// The serial port hasn't been initialized.
struct Uninitialized;
impl SerialStatus for Uninitialized {}
/// The serial port is initialized and ready for use
struct Initialized;
impl SerialStatus for Initialized {}

/// Interface around the serial port
struct SerialPort<S: SerialStatus> {
    port: u16,
    status: PhantomData<S>,
}

impl<S: SerialStatus> SerialPort<S> {
    /// Write a byte of data to a register in the serial port.
    ///
    /// # Safety
    ///
    /// The caller must ensure `register` is a valid serial port register
    unsafe fn write_register(&self, register: u8, data: u8) {
        io::outb(self.port + register as u16, data);
    }

    /// Read a byte of data from a serial port register.
    ///
    /// # Safety
    ///
    /// The caller must ensure `register` is a valid serial port register
    unsafe fn read_register(&self, register: u8) -> u8 {
        io::inb(self.port + register as u16)
    }
}

impl SerialPort<Uninitialized> {
    /// Initialize the serial port and returned the initialized port. Returns `None` if the serial
    /// port failed to initialize.
    ///
    /// # Safety
    ///
    /// The caller must ensure `self.port` is a valid serial port address.
    unsafe fn init(self) -> Option<SerialPort<Initialized>> {
        self.write_register(INTERRUPT_ENABLED, 0); // Disable interrupts
        self.write_register(LINE_CONTROL, 0x80); // Set the DLAB to enable setting the divisor
                                                 // Set the baud rate divisor to 3 (34800 baud)
        self.write_register(BAUD_RATE_LSB, 0x03);
        self.write_register(BAUD_RATE_MSB, 0x00);
        self.write_register(LINE_CONTROL, 0x03); // 8 data bits, 1 stop bit, no parity bits
        self.write_register(FIFO_CONTROL, 0xc7); // Enable FIFO, clear FIFOs, 14-byte threshold
        self.write_register(MODEM_CONTROL, 0x0b); // IRQs enabled, RTS/DSR set

        if !self.self_test() {
            return None;
        }

        self.write_register(MODEM_CONTROL, 0x0f); // Set serial to normal operation mode

        Some(SerialPort::<Initialized> {
            port: self.port,
            status: PhantomData,
        })
    }

    /// Perform a self test on the serial port by setting it to loopback mode and attempting to
    /// write to and read from it
    fn self_test(&self) -> bool {
        // Safety: All registers written to and read from are valid
        unsafe {
            self.write_register(MODEM_CONTROL, 0x1E); // Enable loopback mode
            self.write_register(TRANSMIT_RECIEVE, 0xae);
            self.read_register(TRANSMIT_RECIEVE) == 0xae
        }
    }
}

impl SerialPort<Initialized> {
    /// Write a byte to the serial port
    fn write_byte(&self, byte: u8) {
        self.wait_for_status(LineStatus::TRANSMITTING_HOLDER_REGISTER_EMPTY);
        // Safety: `TRANSMIT_RECIEVE` is a valid register and the line is ready to accept data.
        unsafe {
            self.write_register(TRANSMIT_RECIEVE, byte);
        }
    }

    /// Read a byte off the serial port
    fn read_byte(&self) -> u8 {
        self.wait_for_status(LineStatus::DATA_READY);
        // Safety: `TRANSMIT_RECIEVE` is a valid register and the line has data to be read.
        unsafe { self.read_register(TRANSMIT_RECIEVE) }
    }

    /// Get the line status of the port
    fn get_line_status(&self) -> LineStatus {
        // Safety: LINE_STATUS is a valid register
        LineStatus::from_bits_truncate(unsafe { self.read_register(LINE_STATUS) })
    }

    /// Wait for a specific line status
    fn wait_for_status(&self, status: LineStatus) {
        while !self.get_line_status().contains(status) {
            core::hint::spin_loop();
        }
    }
}

impl fmt::Write for SerialPort<Initialized> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }

        Ok(())
    }
}

/// Initialize the COM 1 port
pub fn init() {
    // Safety: COM 1 is always a valid serial port
    let com_1 = unsafe {
        SerialPort::<Uninitialized> {
            port: COM_1_ADDR,
            status: PhantomData,
        }
        .init()
        .expect("COM 1 failed to initialize")
    };

    COM_1.call_once(|| Mutex::new(com_1));
}

pub macro serial_print($($arg:tt)*) {
    ($crate::drivers::uart_16650::serial_print_internal(format_args!($($arg)*)))
}

pub macro serial_println {
    () => ($crate::drivers::uart_16650::serial_print!("\n")),
    ($($arg:tt)*) => ($crate::drivers::uart_16650::serial_print!("{}\n", format_args!($($arg)*))),
}

#[doc(hidden)]
pub fn serial_print_internal(args: fmt::Arguments) {
    if let Some(c) = COM_1.get() {
        c.lock_irq().write_fmt(args);
    }
}
