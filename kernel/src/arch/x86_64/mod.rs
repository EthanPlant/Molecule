use core::arch::asm;

use interrupts::idt;

use crate::{drivers, logger};

mod gdt;
mod interrupts;
pub mod io;

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum PrivilegeLevel {
    Kernel = 0,
    User = 3,
}

impl From<u8> for PrivilegeLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => PrivilegeLevel::Kernel,
            3 => PrivilegeLevel::User,
            _ => unreachable!("Invalid privilege level"),
        }
    }
}

pub fn arch_init() {
    drivers::uart::init();
    logger::init();
    log::info!("Logger initialized!");

    gdt::init();
    log::debug!("GDT initialized!");

    idt::init();
    log::debug!("IDT initialized!");

    unsafe {
        asm!("sti");
        asm!("int 0x3");
    }

    log::info!("Arch init done!");
}
