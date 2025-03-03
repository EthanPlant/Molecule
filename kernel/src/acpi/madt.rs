use core::mem;

use alloc::vec::Vec;
use spin::RwLock;

use crate::memory::addr::{PhysAddr, VirtAddr};

use super::{SdtHeader, SdtSignature};

pub const MADT_SIG: SdtSignature = SdtSignature(*b"APIC");

pub static IO_APICS: RwLock<Vec<&'static IoApic>> = RwLock::new(Vec::new());
pub static REDIRECTS: RwLock<Vec<&'static IntSrcOverride>> = RwLock::new(Vec::new());

const MADT_HEADER_SIZE: usize = 0x2C;

#[derive(Debug)]
pub struct Madt {
    header: SdtHeader,
    lapic_addr: u32,
    flags: u32,
    addr: usize,
}

impl Madt {
    pub fn new(addr: VirtAddr) -> Self {
        let header = SdtHeader::parse(addr, MADT_SIG).expect("MADT is at this address");
        let lapic_addr =
            unsafe { *((usize::from(addr) + mem::size_of::<SdtHeader>()) as *const u32) };
        let flags = unsafe {
            *((usize::from(addr) + mem::size_of::<SdtHeader>() + mem::size_of::<u32>())
                as *const u32)
        };

        let madt = Self {
            header,
            lapic_addr,
            flags,
            addr: usize::from(addr),
        };

        for entry in madt.iter() {
            match entry {
                MadtEntry::IoApic(ioapic) => {
                    IO_APICS.write().push(ioapic);
                }
                MadtEntry::IntSrcOverride(override_) => {
                    REDIRECTS.write().push(override_);
                }
                _ => {}
            }
        }

        madt
    }

    pub fn iter(&self) -> MadtIter {
        unsafe {
            MadtIter {
                current: (self.addr as *const u8).add(MADT_HEADER_SIZE),
                limit: (self.addr as *const u8).offset(self.header.length as isize),
            }
        }
    }

    pub fn local_apic_addr(&self) -> VirtAddr {
        PhysAddr::new(self.lapic_addr as usize).as_hddm_virt()
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct EntryHeader {
    entry_type: u8,
    length: u8,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct Lapic {
    header: EntryHeader,
    processor_id: u8,
    apic_id: u8,
    flags: u32,
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct IoApic {
    header: EntryHeader,
    ioapic_id: u8,
    reserved: u8,
    ioapic_addr: u32,
    interrupt_base: u32,
}

impl IoApic {
    pub fn addr(&self) -> VirtAddr {
        PhysAddr::new(self.ioapic_addr as usize).as_hddm_virt()
    }

    pub fn interrupt_base(&self) -> u32 {
        self.interrupt_base
    }
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct IntSrcOverride {
    header: EntryHeader,
    bus: u8,
    irq: u8,
    system_int: u32,
    flags: u16,
}

impl IntSrcOverride {
    pub fn irq(&self) -> u8 {
        self.irq
    }

    pub fn system_int(&self) -> u32 {
        self.system_int
    }

    pub fn flags(&self) -> u16 {
        self.flags
    }
}

#[derive(Debug)]
#[repr(C, packed)]
struct IoApicNmi {
    header: EntryHeader,
    nmi: u8,
    reserved: u8,
    flags: u16,
    system_int: u32,
}

#[derive(Debug)]
#[repr(C, packed)]
struct LapicNmi {
    header: EntryHeader,
    processor_id: u8,
    flags: u16,
    lint: u8,
}

#[derive(Debug)]
#[repr(C, packed)]
struct LapicAddrOverride {
    header: EntryHeader,
    reserved: u16,
    addr: u64,
}

#[derive(Debug)]
pub enum MadtEntry {
    Lapic(&'static Lapic),
    IoApic(&'static IoApic),
    IntSrcOverride(&'static IntSrcOverride),
    IoApicNmi(&'static IoApicNmi),
    LapicNmi(&'static LapicNmi),
    LapicAddrOverride(&'static LapicAddrOverride),
}

#[derive(Debug)]
pub struct MadtIter {
    current: *const u8,
    limit: *const u8,
}

impl Iterator for MadtIter {
    type Item = MadtEntry;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.limit {
            unsafe {
                let entry_ptr = self.current;
                let header = *self.current.cast::<EntryHeader>();

                self.current = self.current.offset(header.length as isize);

                let item = match header.entry_type {
                    0 => MadtEntry::Lapic(&*entry_ptr.cast::<Lapic>()),
                    1 => MadtEntry::IoApic(&*entry_ptr.cast::<IoApic>()),
                    2 => MadtEntry::IntSrcOverride(&*entry_ptr.cast::<IntSrcOverride>()),
                    3 => MadtEntry::IoApicNmi(&*entry_ptr.cast::<IoApicNmi>()),
                    4 => MadtEntry::LapicNmi(&*entry_ptr.cast::<LapicNmi>()),
                    5 => MadtEntry::LapicAddrOverride(&*entry_ptr.cast::<LapicAddrOverride>()),

                    0x10..=0x7f => continue,
                    0x80..=0xff => continue,
                    _ => {
                        log::warn!("Unknown MADT entry type found: {}", header.entry_type);
                        return None;
                    }
                };

                return Some(item);
            }
        }

        None
    }
}
