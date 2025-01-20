use crate::{drivers, logger};

pub mod io;

pub fn arch_init() {
    drivers::uart::init();
    logger::init();
    log::info!("Logger initialized!");

    log::info!("Arch init done!");
}
