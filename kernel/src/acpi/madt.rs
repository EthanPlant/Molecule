//! Multiple APIC Description Table (MADT)

use alloc::vec::Vec;
use core::mem;

use spin::{Once, RwLock};

use super::{SdtHeader, SdtSignature};
use crate::memory::addr::{PhysAddr, VirtAddr};

pub(super) const MADT_SIG: SdtSignature = *b"APIC";

/// Global access to the MADT
pub(super) static MADT: Once<&'static Madt> = Once::new();

/// Global access to all IO/APIC entries.
pub static IO_APICS: RwLock<Vec<&'static IoApic>> = RwLock::new(Vec::new());
/// Global access to all interrupt source overrides.
pub static OVERRIDES: RwLock<Vec<&'static IntSrcOverride>> = RwLock::new(Vec::new());

/// In memory representation of a MADT.
#[repr(C, packed)]
pub(super) struct Madt {
    header: SdtHeader,
    lapic_addr: u32,
    flags: u32,
}

impl Madt {
    /// Get the MADT present at `addr`.
    ///
    /// # Safety
    /// `addr` must be a valid pointer to an MADT.
    pub(super) unsafe fn new(addr: VirtAddr) -> &'static Self {
        let this = &*addr.as_ptr::<Madt>();

        for entry in this.iter() {
            match entry {
                MadtEntry::IoApic(e) => IO_APICS.write().push(e),
                MadtEntry::IntSrcOverride(e) => OVERRIDES.write().push(e),
                _ => continue,
            }
        }

        this
    }

    /// Get an iterator over the MADT's entries.
    fn iter(&self) -> MadtIterator {
        // Safety: self points to a valid MADT
        unsafe {
            MadtIterator {
                current: (self as *const Self)
                    .cast::<u8>()
                    .add(mem::size_of::<Self>()),
                limit: (self as *const Self)
                    .cast::<u8>()
                    .offset(self.header.length as isize),
            }
        }
    }
}

/// An iterator over the MADT's entries
struct MadtIterator {
    current: *const u8,
    limit: *const u8,
}

impl Iterator for MadtIterator {
    type Item = MadtEntry;

    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.limit {
            // Safety: current is within the MADT bounds
            unsafe {
                let entry_ptr = self.current;
                let header = &*self.current.cast::<EntryHeader>();

                self.current = self.current.offset(header.length as isize);

                let item = match header.entry_type {
                    0 => MadtEntry::LocalApic(&*entry_ptr.cast()),
                    1 => MadtEntry::IoApic(&*entry_ptr.cast()),
                    2 => MadtEntry::IntSrcOverride(&*entry_ptr.cast()),
                    3 => MadtEntry::IoApicNmi(&*entry_ptr.cast()),
                    4 => MadtEntry::LocalApicNmi(&*entry_ptr.cast()),

                    0x10..=0x7f => continue,
                    0x80..=0xff => continue,

                    _ => {
                        log::warn!("ACPI: Unknown MADT entry with type {}", header.entry_type);

                        return None;
                    }
                };

                return Some(item);
            }
        }

        None
    }
}

/// An entry in the MADT
#[allow(dead_code)]
pub enum MadtEntry {
    LocalApic(&'static ProcessorLocalApic),
    IoApic(&'static IoApic),
    IntSrcOverride(&'static IntSrcOverride),
    IoApicNmi(&'static IoApicNmi),
    LocalApicNmi(&'static LocalApicNmi),
}

/// Header common to all MADT entries.
#[repr(C, packed)]
struct EntryHeader {
    entry_type: u8,
    length: u8,
}

bitflags::bitflags! {
    struct ProcessorLocalApicFlags: u8 {
        /// The processor is ready for use.
        const ENABLED = 1 << 0;
        /// If the enabled bit is set, this bit must be zero. If the enabled bit is clear, this bit is set if the CPU can be enabled
        const ONLINE_CAPABLE = 1 << 1;
    }
}

/// A Processor Local APIC record.
#[repr(C, packed)]
pub struct ProcessorLocalApic {
    header: EntryHeader,
    processor_id: u8,
    apic_id: u8,
    flags: ProcessorLocalApicFlags,
}

/// An I/O APIC record.
#[repr(C, packed)]
pub struct IoApic {
    header: EntryHeader,
    ioapic_id: u8,
    reserved: u8,
    ioapic_addr: u32,
    gsi_base: u32,
}

impl IoApic {
    /// Retrieve the address of this IO/APIC.
    pub fn addr(&self) -> VirtAddr {
        PhysAddr::new(self.ioapic_addr as usize).as_hddm_virt()
    }

    /// Retrieve this IO/APIC's GSI base.
    pub fn gsi_base(&self) -> u32 {
        self.gsi_base
    }
}

/// An Interrupt Source Override record.
#[repr(C, packed)]
pub struct IntSrcOverride {
    header: EntryHeader,
    bus: u8,
    src: u8,
    gsi: u32,
    flags: u16,
}

impl IntSrcOverride {
    /// Retrieve the source IRQ.
    pub fn irq(&self) -> u8 {
        self.src
    }

    /// Retrieve the GSI this interrupt is redirected to.
    pub fn gsi(&self) -> u32 {
        self.gsi
    }

    /// Retrieve the flags for this ISO
    pub fn flags(&self) -> u16 {
        self.flags
    }
}

/// An IO/APIC NMI record.
#[repr(C, packed)]
pub struct IoApicNmi {
    header: EntryHeader,
    flags: u16,
    gsi: u32,
}

/// A Local APIC NMI record
pub struct LocalApicNmi {
    _header: EntryHeader,
    _processor_id: u8,
    _flags: u16,
    _lint: u8,
}
