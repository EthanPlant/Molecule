use crate::{drivers, logger};

mod gdt;
pub mod io;

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum PrivilegeLevel {
    Kernel = 0,
    User = 3,
}

pub fn arch_init() {
    drivers::uart::init();
    logger::init();
    log::info!("Logger initialized!");

    gdt::init();
    log::debug!("GDT initialized!");

    log::info!("Arch init done!");
}
