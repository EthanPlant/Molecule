//! x86_64 initialization.

use super::interrupts::disable_interrupts;
use crate::arch::x86_64::interrupts::idt;
use crate::arch::x86_64::memory::address_space::AddressSpace;
use crate::arch::x86_64::{gdt, paging};
use crate::memory::addr::{VirtAddr, HHDM_OFFSET};
use crate::memory::frame::PhysFrame;
use crate::memory::frame_allocator::{get_frame_allocator, FrameAllocator};
use crate::memory::mem_map;
use crate::memory::page::{Page, Size2M, Size4K};
use crate::{acpi, drivers, logger, memory, HHDM_REQUEST, MEM_MAP_REQUEST, RSDP_REQUEST};

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
    let mut new_addr_space = AddressSpace::new().unwrap();
    new_addr_space.switch();
    let frame = new_addr_space.map_page(
        Page::containing_addr(VirtAddr::new(0xDEADBEEF)),
        get_frame_allocator().allocate_frame().unwrap(),
    );

    // let rsdp_response = RSDP_REQUEST
    //     .get_response()
    //     .expect("Attempting to retrieve RSDP from Limine");
    // acpi::init(rsdp_response);

    crate::kmain()
}
