//! Architecture specific code for ``x86_64``.

use core::sync::atomic::Ordering;

use alloc::{format, string::String};
use interrupts::{
    apic::{self, get_local_apic},
    disable_interrupts, enable_interrupts,
    exception::register_exceptions,
    idt,
};
use limine::smp::Cpu;
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
    hcf, kmain, logger,
    memory::{
        self, addr::{VirtAddr, HHDM_OFFSET}, alloc::init_heap
    },
    HHDM_REQUEST, MEM_MAP_REQUEST, RSDP_REQUEST, SMP_REQUEST,
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
#[no_mangle]
extern "C" fn x86_64_molecule_main() -> ! {
    unsafe { disable_interrupts() };

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

    let smp_response = unsafe {
        SMP_REQUEST
            .get_response_mut()
            .expect("Recieved SMP response from limine")
    };
    let bsp_lapic_id = smp_response.bsp_lapic_id();

    for cpu in smp_response.cpus_mut() {
        apic::CPU_COUNT.fetch_add(1, Ordering::SeqCst);

        if cpu.lapic_id == bsp_lapic_id {
            continue;
        }

        cpu.goto_address.write(ap_main);
    }
    log::info!("CPU count: {}", apic::get_cpu_count());

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

    println!("Welcome to ");
    println!("\x1b[36m  __  __       _                 _      ");
    println!(" |  \\/  |     | |               | |     ");
    println!(" | \\  / | ___ | | ___  ___ _   _| | ___ ");
    println!(" | |\\/| |/ _ \\| |/ _ \\/ __| | | | |/ _ \\");
    println!(" | |  | | (_) | |  __/ (__| |_| | |  __/");
    println!(" |_|  |_|\\___/|_|\\___|\\___|\\__,_|_|\\___|");
    println!("\x1b[0m");
    println!("Version {}", env!("CARGO_PKG_VERSION"));
    println!("CPU Model is {}", cpu_string());
    println!("Total memory: {} MiB", memory::total_memory() / 1024 / 1024);

    apic::set_bsp_ready();

    kmain();
}

extern "C" fn ap_main(cpu: &Cpu) -> ! {
    unsafe { disable_interrupts() };

    let ap_id = cpu.id;
    log::debug!("Initializing CPU {} with LAPIC ID {}", ap_id, cpu.lapic_id);

    gdt::init();
    log::info!("AP {}: GDT initialized!", ap_id);

    idt::init();
    log::info!("AP {}: IDT initialized!", ap_id);

    while !apic::get_bsp_ready() {
        core::hint::spin_loop();
    }

    apic::init_ap();
    log::info!("AP {}: APIC initialized!", ap_id);

    log::debug!("AP {} initialized", ap_id);

    unsafe { enable_interrupts() };

    kmain();
}

pub fn cpu_string() -> String {
    let cpuid = raw_cpuid::CpuId::new();
    let binding = cpuid.get_vendor_info();
    let vendor = binding.as_ref().map_or_else(|| "unknown", |vf| vf.as_str());
    let binding = cpuid.get_processor_brand_string();
    let model = binding.as_ref().map_or_else(|| "unknown", |s| s.as_str());
    format!("{vendor} {model}")
}
