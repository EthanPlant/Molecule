//! x86_64 initialization.

use super::interrupts::disable_interrupts;
use crate::arch::x86_64::interrupts::idt;
use crate::arch::x86_64::{gdt, paging};
use crate::memory::addr::{VirtAddr, HHDM_OFFSET};
use crate::{acpi, drivers, logger, HHDM_REQUEST, MEM_MAP_REQUEST, RSDP_REQUEST};

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
    log::debug!("HHDM Offset: {:x?}", HHDM_OFFSET.get().unwrap());

    gdt::init();
    idt::init();

    unsafe { core::arch::asm!("int 0x0") };

    let mem_map_response = unsafe {
        MEM_MAP_REQUEST
            .get_response_mut()
            .expect("Attempting to retrieve memory map from Limine")
    };

    // let rsdp_response = RSDP_REQUEST
    //     .get_response()
    //     .expect("Attempting to retrieve RSDP from Limine");
    // acpi::init(rsdp_response);

    crate::kmain()
}
