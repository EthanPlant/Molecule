use crate::memory::addr::VirtAddr;

use core::mem;

use alloc::{borrow::ToOwned, vec::Vec};

use crate::memory::addr::PhysAddr;

use super::{SdtHeader, SdtSignature};

const XSDT_SIG: SdtSignature = SdtSignature(*b"XSDT");
const RSDT_SIG: SdtSignature = SdtSignature(*b"RSDT");

#[derive(Copy, Clone, Debug)]
pub enum RsdtAddr {
    Xsdt(VirtAddr),
    Rsdt(VirtAddr),
}

#[derive(Copy, Clone, Debug)]
pub struct RsdtEntry {
    signature: SdtSignature,
    addr: VirtAddr,
}

impl RsdtEntry {
    pub fn addr(&self) -> VirtAddr {
        self.addr
    }
}

#[derive(Clone, Debug)]
pub struct Rsdt {
    header: SdtHeader,
    entries: usize,
    sub_tables: Vec<RsdtEntry>,
    addr_width: usize,
}

impl Rsdt {
    pub fn new(addr: RsdtAddr) -> Self {
        match addr {
            RsdtAddr::Xsdt(xsdt_addr) => {
                let header =
                    SdtHeader::parse(xsdt_addr, XSDT_SIG).expect("XSDT should be at this address");
                let entries = (header.length as usize - mem::size_of::<SdtHeader>()) / 8;
                let sub_tables =
                    populate_sub_tables(xsdt_addr + mem::size_of::<SdtHeader>(), entries, 64);
                Self {
                    header,
                    entries,
                    sub_tables,
                    addr_width: 64,
                }
            }
            RsdtAddr::Rsdt(rsdt_addr) => {
                let header =
                    SdtHeader::parse(rsdt_addr, RSDT_SIG).expect("RSDT should be at this address");
                let entries = (header.length as usize - mem::size_of::<SdtHeader>()) / 4;
                let sub_tables =
                    populate_sub_tables(rsdt_addr + mem::size_of::<SdtHeader>(), entries, 32);
                Self {
                    header,
                    entries,
                    sub_tables,
                    addr_width: 32,
                }
            }
        }
    }

    pub fn find_table(&self, signature: SdtSignature) -> Option<RsdtEntry> {
        self.sub_tables
            .iter()
            .filter(|&&table| table.signature == signature)
            .next()
            .copied()
    }
}

fn populate_sub_tables(addr: VirtAddr, entries: usize, width: usize) -> Vec<RsdtEntry> {
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
        sub_tables.push(RsdtEntry {
            signature: header.signature,
            addr: PhysAddr::new(ptr).as_hddm_virt(),
        });
    }

    sub_tables
}
