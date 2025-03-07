//! Root/Extended System Descriptor Table (R/XSDT)

use alloc::vec::Vec;
use core::marker::PhantomData;
use core::mem;

use super::{SdtHeader, SdtSignature};
use crate::memory::addr::{PhysAddr, VirtAddr};

/// Contains an address to either an XSDT or RSDT.
pub(super) enum RsdtAddr {
    Xsdt(VirtAddr),
    Rsdt(VirtAddr),
}

/// Addressing width for an RSDT. An RSDT can either use 32-bit addressing or 64-bit addressing in
/// the case of an XSDT.
pub(super) trait RsdtType {}
impl RsdtType for u32 {}
impl RsdtType for u64 {}

/// In memory representation of the RSDT. The RSDT or XSDT contains the location of all other ACPI
/// tables.
pub(super) struct Rsdt<T: RsdtType + Sized> {
    tables: Vec<RsdtEntry>,
    _phantom: PhantomData<T>,
}

/// An entry in the RSDT.
pub(super) struct RsdtEntry {
    signature: SdtSignature,
    addr: VirtAddr,
}

impl RsdtEntry {
    /// Returns the address of this entry's table.
    pub fn addr(&self) -> VirtAddr {
        self.addr
    }
}

// Implementation for a RSDT with 32-bit addressing
impl Rsdt<u32> {
    /// Construct a new RSDT from an address.
    ///
    /// # Safety:
    /// The address must be a valid pointer to a 32-bit RSDT.
    pub unsafe fn new(addr: VirtAddr) -> Self {
        let header = &*addr.as_ptr::<SdtHeader>();
        let entries =
            (header.length as usize + mem::size_of::<SdtHeader>()) / mem::size_of::<u32>();
        let tables = Self::populate_tables(addr + mem::size_of::<SdtHeader>(), entries);
        Self {
            tables,
            _phantom: PhantomData,
        }
    }

    /// Find each table pointed to by the RSDT.
    ///
    /// # Safety:
    /// Undefined behaviour can occur if any of the following conditions are violated
    /// - `addr` must point to the beginning of the list of sub tables in the RSDT
    /// - The RSDT must have at most `entries` entries.
    unsafe fn populate_tables(addr: VirtAddr, entries: usize) -> Vec<RsdtEntry> {
        let mut tables = Vec::with_capacity(entries);

        for i in 0..entries {
            let table_addr = core::ptr::read_unaligned(addr.as_ptr::<u32>().add(i));
            let addr = PhysAddr::new(table_addr as usize).as_hddm_virt();
            let sdt_header = &*addr.as_ptr::<SdtHeader>();
            tables.push(RsdtEntry {
                signature: sdt_header.signature,
                addr,
            });
        }

        tables
    }
}

// Implementation for an XSDT with 64-bit addressing
impl Rsdt<u64> {
    /// Construct a new XSDT from an address.
    ///
    /// # Safety:
    /// The address must be a valid pointer to a 64 bit XSDT.
    pub unsafe fn new(addr: VirtAddr) -> Self {
        let header = &*addr.as_ptr::<SdtHeader>();
        let entries =
            (header.length as usize - mem::size_of::<SdtHeader>()) / mem::size_of::<u64>();
        let tables = Self::populate_tables(addr + mem::size_of::<SdtHeader>(), entries);
        Self {
            tables,
            _phantom: PhantomData,
        }
    }

    /// Find each table pointed to by the XSDT.
    ///
    /// # Safety:
    /// Undefined behaviour can occur if any of the following conditions are violated
    /// - `addr` must point to the beginning of the list of sub tables in the XSDT
    /// - The XSDT must have at most `entries` entries.
    unsafe fn populate_tables(addr: VirtAddr, entries: usize) -> Vec<RsdtEntry> {
        let mut tables = Vec::with_capacity(entries);

        for i in 0..entries {
            let table_addr = core::ptr::read_unaligned(addr.as_ptr::<u64>().add(i));
            let addr = PhysAddr::new(table_addr as usize).as_hddm_virt();
            let sdt_header = &*addr.as_ptr::<SdtHeader>();
            tables.push(RsdtEntry {
                signature: sdt_header.signature,
                addr,
            });
        }

        tables
    }
}

impl<T: RsdtType> Rsdt<T> {
    /// Find a table with a given signature from the RSDT. Returns `None` if the requested table
    /// isn't present.
    pub fn find_table(&self, signature: SdtSignature) -> Option<&RsdtEntry> {
        self.tables
            .iter()
            .find(|&table| table.signature == signature)
    }
}
