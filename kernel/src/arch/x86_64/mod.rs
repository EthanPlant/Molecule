//! Architecture specific code for ``x86_64``.

use interrupts::{
    exception::register_exceptions,
    idt,
};

use crate::{
    drivers, logger,
    memory::addr::{VirtAddr, HHDM_OFFSET},
    HHDM_REQUEST,
};

mod gdt;
mod interrupts;
pub mod io;

/// Represents the privilege level of the CPU.
#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(u8)]
pub enum PrivilegeLevel {
    /// Ring 0 or kernel privilege level.
    Kernel = 0,
    /// Ring 3 or user privilege level.
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

/// Performs any ``x86_64`` specific initialization.
///
/// This function is called during the kernel initialization process to perform any architecture specific initialization.
/// Specifically, this function performs the following:
/// 1. Initializes the [UART driver](drivers::uart_16650) for serial logging.
/// 2. Initializes the [GDT](gdt) (Global Descriptor Table).
/// 3. Initializes the [IDT](idt) (Interrupt Descriptor Table).
/// 4. Registers handlers for CPU exceptions.
pub fn arch_init() {
    drivers::uart::init();
    logger::init();
    log::info!("Logger initialized!");

    HHDM_OFFSET.call_once(|| {
        VirtAddr::new(
            HHDM_REQUEST
                .get_response()
                .expect("Limine should return HHDM offset")
                .offset() as usize,
        )
    });

    gdt::init();
    log::debug!("GDT initialized!");

    idt::init();
    log::debug!("IDT initialized!");

    register_exceptions();
    log::debug!("Exceptions registered!");

    log::info!("Arch init done!");
}
