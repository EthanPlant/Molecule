//! x86_64 initialization.

use core::sync::atomic::Ordering;

use limine::smp::Cpu;

use super::interrupts::disable_interrupts;
use crate::arch::interrupts::apic::get_local_apic;
use crate::arch::interrupts::{apic, enable_interrupts};
use crate::arch::x86_64::interrupts::idt;
use crate::arch::x86_64::memory::heap;
use crate::arch::x86_64::{gdt, time};
use crate::drivers::framebuffer;
use crate::memory::addr::{VirtAddr, HHDM_OFFSET};
use crate::memory::frame::PhysFrame;
use crate::memory::frame_allocator::{get_frame_allocator, FrameAllocator};
use crate::memory::mem_map;
use crate::memory::page::{Page, Size2M, Size4K};
use crate::{
    acpi, ap_kmain, drivers, logger, memory, FRAMEBUFFER_REQUEST, HHDM_REQUEST, MEM_MAP_REQUEST,
    RSDP_REQUEST, SMP_REQUEST,
};

/// Kernel entry point. This function performs early initialization for the kernel's subsystems
/// before passing control over to [crate::kmain].
#[no_mangle]
extern "C" fn x86_64_molecule_main() -> ! {
    disable_interrupts();

    drivers::uart::init();
    logger::init();
    log::info!("Init: Serial logger initialized");

    HHDM_OFFSET.call_once(|| {
        VirtAddr::new(
            HHDM_REQUEST
                .get_response()
                .expect("Attempting to get HHDM offset from Limine")
                .offset() as usize,
        )
    });
    log::debug!("HHDM Offset: {:x}", HHDM_OFFSET.get().unwrap());

    gdt::init();
    idt::init();

    let mem_map_response = unsafe {
        MEM_MAP_REQUEST
            .get_response_mut()
            .expect("Attempting to retrieve memory map from Limine")
    };

    memory::frame_allocator::init(mem_map_response);
    heap::init().expect("Attempting to allocate heap");

    let fb_resp = FRAMEBUFFER_REQUEST
        .get_response()
        .expect("Attempting to retrieve framebuffer from Limine");
    framebuffer::init(fb_resp);

    let rsdp_response = RSDP_REQUEST
        .get_response()
        .expect("Attempting to retrieve RSDP from Limine");
    acpi::init(rsdp_response);

    let smp_response = unsafe {
        SMP_REQUEST
            .get_response_mut()
            .expect("Attempting to retrieve SMP response from Limine")
    };

    let bsp_id = smp_response.bsp_lapic_id();
    for cpu in smp_response.cpus_mut() {
        apic::CPU_COUNT.fetch_add(1, Ordering::SeqCst);
        if cpu.lapic_id != bsp_id {
            cpu.goto_address.write(ap_init);
        }
    }

    apic::init();
    time::init();

    crate::kmain()
}

extern "C" fn ap_init(cpu: &Cpu) -> ! {
    disable_interrupts();

    let ap_id = cpu.id;
    log::debug!("Iniailizing CPU {ap_id} with LAPIC ID {}", cpu.lapic_id);

    gdt::init();
    log::info!("AP {ap_id}: GDT Initialized");

    idt::init();
    log::info!("AP {ap_id}: IDT Initialized");

    while !apic::get_bsp_ready() {
        core::hint::spin_loop();
    }

    unsafe { get_local_apic().init() };
    log::info!("AP {ap_id}: APIC Initialized");

    time::init_ap();

    ap_kmain(ap_id);
}
