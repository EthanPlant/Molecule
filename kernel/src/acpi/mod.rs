//! # ACPI Interface
//!
//! This module contains the interface used to provide access to the Advanced Configuration and
//! Power Interface (ACPI) data structures.

use hpet::{HpetTable, HPET_SIG};
use madt::{Madt, MADT, MADT_SIG};
use rsdt::{Rsdt, RsdtAddr, RsdtType};

pub mod hpet;
pub mod madt;
mod rsdp;
mod rsdt;

type SdtSignature = [u8; 4];

/// Header data present in all ACPI tables. This header data contains metadata about the table
/// itself.
#[repr(C, packed)]
#[derive(Clone)]
struct SdtHeader {
    signature: SdtSignature,
    length: u32,
    revision: u8,
    checksum: u8,
    oem_id: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

const GENERIC_ADDR_IN_MEM: u8 = 0;

/// Generic Address Structure: Describes register addresses within ACPI tables.
#[repr(C, packed)]
struct GenericAddress {
    addr_space_id: u8,
    register_bit_width: u8,
    register_bit_offset: u8,
    access_size: u8,
    address: u64,
}

/// Initialize the ACPI tables from a [RsdpResponse](limine::response::RsdpResponse)
pub fn init(resp: &limine::response::RsdpResponse) {
    log::info!("ACPI: Beginning initialization");
    match rsdp::find_rsdt_addr(resp) {
        RsdtAddr::Xsdt(xsdt_addr) => {
            log::debug!("ACPI: XSDT found at {:x?}", xsdt_addr);
            // Safety: The XSDT address from a V2 RSDP is guaranteed to be valid
            let xsdt = unsafe { Rsdt::<u64>::new(xsdt_addr) };
            init_inner(&xsdt);
        }
        RsdtAddr::Rsdt(rsdt_addr) => {
            log::debug!("ACPI: RSDT found at {:x?}", rsdt_addr);
            // Safety: The RSDT address from a V1 RSDP is guaranteed to be valid
            let rsdt = unsafe { Rsdt::<u32>::new(rsdt_addr) };
            init_inner(&rsdt);
        }
    }
    log::info!("ACPI: Initialization finished")
}

fn init_inner<T: RsdtType>(rsdt: &Rsdt<T>) {
    if let Some(madt_entry) = rsdt.find_table(MADT_SIG) {
        log::debug!("ACPI: MADT found at {:x?}", madt_entry.addr());
        // Safety: MADT address came from the RSDT, and must be valid
        MADT.call_once(|| unsafe { Madt::new(madt_entry.addr()) });
    } else {
        log::warn!("ACPI: No MADT found in RSDT")
    }

    if let Some(hpet_entry) = rsdt.find_table(HPET_SIG) {
        log::debug!("ACPI: HPET found at {:x?}", hpet_entry.addr());
        // Safety: HPET address came from the RSDT, and must be valid
        hpet::init(unsafe { HpetTable::new(hpet_entry.addr()) });
    }
}
