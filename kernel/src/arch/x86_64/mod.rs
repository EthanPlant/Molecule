use crate::{drivers::{self, uart_16650::serial_println}, logger};

pub mod io;

pub fn arch_init() {
    drivers::uart::init();
    logger::init();
    log::info!("Logger initialized!");

    log::info!("Arch init done!");
}