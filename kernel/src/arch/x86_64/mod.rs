use crate::drivers::{self, uart_16650::serial_println};
use core::fmt::Write;

pub mod io;

pub fn arch_init() {
    drivers::uart::init();

    serial_println!("Hello, world!");
}