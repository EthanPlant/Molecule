#![feature(decl_macro)]
#![feature(naked_functions)]
#![feature(allocator_api)]
#![feature(strict_provenance_atomic_ptr)]
#![feature(ptr_internals)]
#![feature(abi_x86_interrupt)]
#![feature(maybe_uninit_slice)]
#![feature(trait_upcasting)]
#![no_std]
#![no_main]
#![deny(trivial_numeric_casts, unused_allocation)]
#![warn(clippy::needless_pass_by_value)]
#![warn(clippy::ptr_as_ptr)]
#![warn(missing_copy_implementations)]
#![allow(internal_features)]

//! The Molecule kernel.
use core::arch::asm;
use core::str;

use arch::interrupts::apic::set_bsp_ready;
use arch::interrupts::enable_interrupts;
use drivers::framebuffer::console::println;
use fs::path::Path;
use fs::vfs;
use fs::vfs::resolver::ResolutionSettings;
// use fs::path::Path;
// use fs::perm::AccessProfile;
// use fs::vfs::resolver::ResolutionSettings;
// use fs::{vfs, Stat};
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, ModuleRequest, RequestsEndMarker,
    RequestsStartMarker, RsdpRequest, SmpRequest,
};
use limine::BaseRevision;
use linked_list_allocator::LockedHeap;
use memory::frame_allocator::get_frame_allocator;
use process::scheduler;
extern crate alloc;

mod acpi;
mod arch;
mod drivers;
mod fs;
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
pub static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[used]
#[link_section = ".requests"]
pub static mut MEM_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[used]
#[link_section = ".requests"]
pub static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

#[used]
#[link_section = ".requests"]
pub static mut SMP_REQUEST: SmpRequest = SmpRequest::new();

#[used]
#[link_section = ".requests"]
static MODULE_REQUEST: ModuleRequest = ModuleRequest::new();

/// Define the stand and end markers for Limine requests.
#[used]
#[link_section = ".requests_start_marker"]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();
#[used]
#[link_section = ".requests_end_marker"]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[global_allocator]
pub static GLOBAL_ALLOC: LockedHeap = LockedHeap::empty();

pub static mut TICKS: usize = 0;

pub fn kmain() -> ! {
    log::info!("Starting Molecule {}", env!("CARGO_PKG_VERSION"));

    scheduler::init();

    fs::init();
    let modules = MODULE_REQUEST
        .get_response()
        .expect("Failed to retrieve modules from Limine")
        .modules();
    let mut initramfs = None;
    for &module in modules {
        let path_str = core::str::from_utf8(module.path());
        if let Ok(path) = path_str {
            if path == "/boot/initramfs" {
                initramfs = Some(module);
            }
        }
    }
    if let Some(initramfs) = initramfs {
        log::info!("Initialzing initramfs");
        let res = fs::initramfs::load(initramfs);
        if res.is_err() {
            log::warn!("Error loading initramfs: {:?}", res);
        }
    }

    println!("Welcome to Molecule!");
    println!(
        "Kernel ver: {} (commit = {}), built for {}",
        env!("CARGO_PKG_VERSION"),
        env!("COMMIT"),
        env!("TARGET")
    );
    println!(
        "{} MiB free",
        get_frame_allocator().get_total_memory() / 1024 / 1024
    );

    let paths = [
        Path::new("hi.txt").unwrap(),
        Path::new("b.txt").unwrap(),
        Path::new("subdir/sub.txt").unwrap(),
    ];
    for path in paths {
        println!("Content of {}", path);
        let file = vfs::get_file_from_path(path, &ResolutionSettings::kernel_no_follow());
        if file.is_ok() {
            let content = file.unwrap().read_all().unwrap();
            let txt = str::from_utf8(&content).unwrap();
            println!("{txt}");
        }
    }

    #[cfg(target_arch = "x86_64")]
    set_bsp_ready();

    enable_interrupts();

    hcf();
}

pub fn ap_kmain(ap: u32) -> ! {
    log::trace!("CPU {ap} initialized");
    enable_interrupts();

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
            asm!("sti; hlt");
            #[cfg(any(target_arch = "aarch64", target_arch = "riscv64"))]
            asm!("wfi");
            #[cfg(target_arch = "loongarch64")]
            asm!("idle 0");
        }
    }
}
