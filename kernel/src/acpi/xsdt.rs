use core::mem;

use alloc::vec::Vec;

use crate::memory::addr::{PhysAddr, VirtAddr};

use super::{rsdp::Rsdp, SdtHeader, SdtSignature};

#[derive(Copy, Clone, Debug)]
struct XsdtEntry {
    signature: SdtSignature,
    addr: VirtAddr,
}

#[derive(Clone, Debug)]
pub struct Xsdt {
    header: SdtHeader,
    entries: usize,
    sub_tables: Vec<XsdtEntry>,
    addr_width: usize,
}

impl Xsdt {
    pub fn new(rsdp: &Rsdp) -> Self {
        let addr = if rsdp.revision() != 0 {
            rsdp.xsdt_addr()
                .expect("ACPI version 2 tables will always have a valid XSDT address")
        } else {
            rsdp.rsdt_addr()
        }
        .as_hddm_virt();

        if let Some(header) = SdtHeader::parse(addr, SdtSignature(*b"XSDT")) {
            let entries = (header.length as usize - mem::size_of::<SdtHeader>()) / 8;
            let sub_tables = populate_sub_tables(addr + mem::size_of::<SdtHeader>(), entries, 64);
            return Self {
                header,
                entries,
                sub_tables,
                addr_width: 64,
            };
        }
        assert!(
            (rsdp.revision() != 0),
            "ACPI 2.0 detected, no valid XSDT found!"
        );

        if let Some(header) = SdtHeader::parse(addr, SdtSignature(*b"RSDT")) {
            let entries = (header.length as usize - mem::size_of::<SdtHeader>()) / 4;
            let sub_tables = populate_sub_tables(addr + mem::size_of::<SdtHeader>(), entries, 32);
            return Self {
                header,
                entries,
                sub_tables,
                addr_width: 32,
            };
        }

        unreachable!()
    }
}

fn populate_sub_tables(addr: VirtAddr, entries: usize, width: usize) -> Vec<XsdtEntry> {
    let mut sub_tables = Vec::with_capacity(entries);
    for i in 0..entries {
        let mut ptr: usize = 0;
        let ptr_low = unsafe { *((usize::from(addr) + i * (width / 8)) as *const u32) };
        ptr |= ptr_low as usize;
        if width == 64 {
            let ptr_high = unsafe { *((usize::from(addr) + i * (width / 8) + 4) as *const u32) };
            ptr |= (ptr_high as usize) << 32;
        }
        let header = SdtHeader::parse_from_addr(PhysAddr::new(ptr).as_hddm_virt());
        sub_tables.push(XsdtEntry {
            signature: header.signature,
            addr: PhysAddr::new(ptr).as_hddm_virt(),
        });
    }

    sub_tables
}
