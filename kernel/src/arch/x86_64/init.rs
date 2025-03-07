//! x86_64 initialization.

use crate::{arch::x86_64::{gdt, interrupts::idt, paging}, drivers, logger, memory::addr::{VirtAddr, HHDM_OFFSET}, HHDM_REQUEST, MEM_MAP_REQUEST};

use super::interrupts::disable_interrupts;

/// Kernel entry point. This function performs early initialization for the kernel's subsystems before passing control over to [crate::kmain].
#[no_mangle]
extern "C" fn x86_64_molecule_main() -> ! {
    disable_interrupts();

    drivers::uart::init();
    logger::init();
    log::info!("Init: Serial logger initialized");

    HHDM_OFFSET.call_once(|| {
        VirtAddr::new(HHDM_REQUEST
            .get_response()
            .expect("Attempting to get HHDM offset from Limine")
            .offset() as usize
        )
    });
    log::debug!("HHDM Offset: {:x?}", HHDM_OFFSET.get().unwrap());

    gdt::init();
    idt::init();

    let mem_map_response = unsafe {
        MEM_MAP_REQUEST
            .get_response_mut()
            .expect("Attempting to retrieve memory map from Limine")
    };

    crate::kmain()
}