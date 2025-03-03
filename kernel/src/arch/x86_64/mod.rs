//! Architecture specific code for ``x86_64``.

use alloc::{format, string::String};
use interrupts::{
    apic::{self, get_local_apic},
    enable_interrupts,
    exception::register_exceptions,
    idt,
};
use paging::page_table::active_level_4_table;

use crate::{
    acpi::{
        hpet,
        rsdp::{self, Rsdp},
        rsdt::{self, Rsdt},
        ACPI_TABLES,
    },
    drivers::{
        self,
        framebuffer::{self, color::Color, console::println, framebuffer},
        uart_16650::serial_println,
    },
    logger,
    memory::{
        addr::{VirtAddr, HHDM_OFFSET},
        alloc::init_heap,
    },
    HHDM_REQUEST, MEM_MAP_REQUEST, RSDP_REQUEST,
};

mod gdt;
pub mod interrupts;
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
    unsafe {
        core::arch::asm!("cli");
    }

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

    log::debug!("HHDM Offset: {:x?}", HHDM_OFFSET.get());

    gdt::init();
    log::info!("GDT initialized!");
    log::debug!("GDT Address: {:x?}", gdt::gdt_addr());

    idt::init();
    log::info!("IDT initialized!");
    log::debug!("IDT Address: {:x?}", idt::idt_addr());

    register_exceptions();

    #[allow(static_mut_refs)]
    let mem_map_response = unsafe {
        MEM_MAP_REQUEST
            .get_response_mut()
            .expect("Didn't recieve memory map response from limine")
    };
    paging::init(mem_map_response);
    log::info!("Initialized memory manager");

    init_heap(unsafe { &mut active_level_4_table() })
        .expect("Failed to allocate space for kernel heap!");
    log::info!("Heap initialized");

    framebuffer::init();
    framebuffer().clear_screen(Color::BLACK);
    log::info!("Framebuffer console initialized, all further messages will be displayed");

    hpet::init_hpet(ACPI_TABLES.hpet());

    apic::init();
    log::info!("APIC initialized");

    unsafe { enable_interrupts() };

    log::info!("Arch init done!");
}

pub fn cpu_string() -> String {
    let cpuid = raw_cpuid::CpuId::new();
    let binding = cpuid.get_vendor_info();
    let vendor = binding.as_ref().map_or_else(|| "unknown", |vf| vf.as_str());
    let binding = cpuid.get_processor_brand_string();
    let model = binding.as_ref().map_or_else(|| "unknown", |s| s.as_str());
    format!("{vendor} {model}")
}
