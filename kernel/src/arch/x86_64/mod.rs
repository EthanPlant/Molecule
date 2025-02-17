//! Architecture specific code for ``x86_64``.

use interrupts::{apic, exception::register_exceptions, idt};
use paging::page_table::active_level_4_table;

use crate::{
    acpi::{
        rsdp::{self, Rsdp},
        rsdt::{self, Rsdt},
        ACPI_TABLES,
    },
    drivers::{
        self,
        framebuffer::{self, color::Color, framebuffer},
    },
    logger,
    memory::{
        addr::{VirtAddr, HHDM_OFFSET},
        alloc::init_heap,
    },
    HHDM_REQUEST, MEM_MAP_REQUEST, RSDP_REQUEST,
};

mod gdt;
mod interrupts;
pub mod io;
pub mod paging;

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
/// 5. Initialize memory management and the heap allocator.
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

    #[allow(static_mut_refs)]
    let mem_map_response = unsafe {
        MEM_MAP_REQUEST
            .get_response_mut()
            .expect("Didn't recieve memory map response from limine")
    };
    paging::init(mem_map_response);
    log::debug!("Initialized memory manager");

    init_heap(unsafe { &mut active_level_4_table() })
        .expect("Failed to allocate space for kernel heap!");
    log::debug!("Heap initialized");

    framebuffer::init();
    framebuffer().clear_screen(Color::BLACK);
    log::info!("Console initialized, all further messages will be displayed");

    let apic_type = apic::init();
    log::debug!("APIC Type: {:?}", apic_type);

    log::debug!("{:x?}", ACPI_TABLES.rsdt());
    log::debug!("{:x?}", ACPI_TABLES.madt().iter());

    for entry in ACPI_TABLES.madt().iter() {
        log::debug!("MADT Entry {:x?}", entry);
    }

    log::info!("Arch init done!");
}
