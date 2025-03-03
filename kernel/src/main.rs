#![feature(decl_macro)]
#![feature(naked_functions)]
#![feature(allocator_api)]
#![feature(strict_provenance_atomic_ptr)]
#![feature(ptr_internals)]
#![no_std]
#![no_main]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]

//! The Molecule kernel.

use core::arch::asm;

use alloc::sync::Arc;
use arch::interrupts::apic::get_local_apic;
use drivers::framebuffer::color::Color;
use drivers::framebuffer::console::{print, println};
use drivers::framebuffer::{self, framebuffer};
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, RequestsEndMarker, RequestsStartMarker,
    RsdpRequest, SmpRequest,
};
use limine::BaseRevision;
use linked_list_allocator::LockedHeap;
use process::Process;
use psf::PsfFont;

extern crate alloc;

mod acpi;
mod arch;
mod drivers;
mod logger;
mod memory;
mod process;
mod psf;
mod sync;

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
#[allow(missing_docs)]
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".requests"]
#[allow(missing_docs)]
pub static mut MEM_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[link_section = ".requests"]
#[allow(missing_docs)]
pub static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

#[used]
#[link_section = ".requests"]
#[allow(missing_docs)]
pub static mut SMP_REQUEST: SmpRequest = SmpRequest::new();

/// Define the stand and end markers for Limine requests.
#[used]
#[link_section = ".requests_start_marker"]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();
#[used]
#[link_section = ".requests_end_marker"]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[global_allocator]
#[allow(missing_docs)]
pub static GLOBAL_ALLOC: LockedHeap = LockedHeap::empty();

pub static mut TICKS: usize = 0;

pub fn kmain() -> ! {
    log::info!("Starting Molecule {}", env!("CARGO_PKG_VERSION"));

    // All limine requests must also be referenced in a called function, otherwise they may be
    // removed by the linker.
    assert!(BASE_REVISION.is_supported());

    unsafe {
        core::arch::asm!("int 0x80");
    }

    hcf();
}

pub fn kernel_task() {
    log::trace!("Hi from a process!");
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
