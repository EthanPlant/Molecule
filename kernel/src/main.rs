#![feature(decl_macro)]
#![feature(naked_functions)]
#![feature(allocator_api)]
#![no_std]
#![no_main]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]

//! The Molecule kernel.

use core::arch::asm;

use alloc::boxed::Box;
use arch::arch_init;
use dummy_alloc::DummyAlloc;
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, RequestsEndMarker, RequestsStartMarker,
};
use limine::BaseRevision;
use memory::bootstrap::{BootstrapAlloc, BootstrapAllocRef};
use memory::frame::{BuddyFrameAllocator, FrameAllocator, FRAME_ALLOCATOR};
use memory::memmap::MemoryRegionIter;

extern crate alloc;

mod arch;
mod drivers;
mod logger;
mod memory;

/// Sets the base revision to the latest revision supported by the crate.
/// See specification for further info.
/// Be sure to mark all limine requests with #[used], otherwise they may be removed by the compiler.
#[used]
// The .requests section allows limine to find the requests faster and more safely.
#[link_section = ".requests"]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[link_section = ".requests"]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[link_section = ".requests"]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".requests"]
pub static mut MEM_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

/// Define the stand and end markers for Limine requests.
#[used]
#[link_section = ".requests_start_marker"]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();
#[used]
#[link_section = ".requests_end_marker"]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[global_allocator]
static GLOBAL_ALLOC: DummyAlloc = DummyAlloc;

#[no_mangle]
unsafe extern "C" fn kmain() -> ! {
    arch_init();

    log::info!("Dropped into kmain!");
    log::info!("Running Molecule {}", env!("CARGO_PKG_VERSION"));

    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());

    log::debug!(
        "HHDM Address: {:x}",
        HHDM_REQUEST.get_response().unwrap().offset()
    );

    if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
        if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
            for i in 0..100_u64 {
                // Calculate the pixel offset using the framebuffer information we obtained above.
                // We skip `i` scanlines (pitch is provided in bytes) and add `i * 4` to skip `i` pixels forward.
                let pixel_offset = i * framebuffer.pitch() + i * 4;

                // Write 0xFFFFFFFF to the provided pixel offset to fill it white.
                *(framebuffer.addr().add(pixel_offset as usize) as *mut u32) = 0xFFFFFFFF;
            }
        }
    }

    hcf();
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    log::error!("{}", info);
    hcf();
}

fn hcf() -> ! {
    loop {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            asm!("hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            asm!("idle 0");
        }
    }
}
