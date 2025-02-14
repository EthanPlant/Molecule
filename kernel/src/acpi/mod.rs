use core::{
    fmt::{self, Debug},
    str,
};

use alloc::fmt::format;
use rsdp::{find_rsdt_addr, Rsdp};
use rsdt::Rsdt;
use spin::Lazy;

use crate::{memory::addr::VirtAddr, RSDP_REQUEST};

pub mod rsdp;
pub mod rsdt;

pub static ACPI_TABLES: Lazy<AcpiTables> = Lazy::new(|| {
    init(
        RSDP_REQUEST
            .get_response()
            .expect("RSDP response returned from Limine"),
    )
});

#[derive(Debug)]
pub struct AcpiTables {
    rsdt: Rsdt,
}

impl AcpiTables {
    pub fn rsdt(&self) -> &Rsdt {
        &self.rsdt
    }
}

pub fn init(resp: &limine::response::RsdpResponse) -> AcpiTables {
    let addr = VirtAddr::new(resp.address() as usize);
    let rsdt_addr = find_rsdt_addr(addr);
    log::debug!("RSDT found at {:x?}", rsdt_addr);
    let rsdt = Rsdt::new(rsdt_addr);

    AcpiTables { rsdt }
}

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
struct SdtSignature([u8; 4]);

impl fmt::Debug for SdtSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}", unsafe {
            &str::from_utf8_unchecked(&self.0)
        }))
    }
}

#[derive(Clone, Copy)]
#[repr(C, packed)]
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

impl SdtHeader {
    pub fn parse(addr: VirtAddr, signature: SdtSignature) -> Option<Self> {
        let header = unsafe { &*(usize::from(addr) as *const SdtHeader) };
        if header.signature == signature && header.validate_checksum() {
            return Some(*header);
        }

        None
    }

    pub fn parse_from_addr(addr: VirtAddr) -> Self {
        let header = unsafe { &*(usize::from(addr) as *const SdtHeader) };
        if header.validate_checksum() {
            return *header;
        }

        unreachable!()
    }

    fn validate_checksum(&self) -> bool {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                core::ptr::from_ref::<SdtHeader>(self).cast::<u8>(),
                self.length as usize,
            )
        };
        let sum = bytes.iter().fold(0u8, |sum, &byte| sum.wrapping_add(byte));
        sum == 0
    }
}

impl fmt::Debug for SdtHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("SdtHeader");
        let length = self.length;
        let oem_revision = self.oem_revision;
        let creator_id = self.creator_id;
        let creator_revision = self.creator_revision;
        f.field("signature", &self.signature);
        f.field("length", &length);
        f.field("revision", &self.revision);
        f.field("checksum", &self.checksum);
        f.field("oem_id", unsafe { &str::from_utf8_unchecked(&self.oem_id) });
        f.field("oem_table_id", unsafe {
            &str::from_utf8_unchecked(&self.oem_table_id)
        });
        f.field("oem_revision", &oem_revision);
        f.field("creator_id", &creator_id);
        f.field("creator_revision", &creator_revision);
        f.finish()
    }
}
