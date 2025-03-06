//! High Precision Event Timer (HPET)

use spin::Once;

use super::{GenericAddress, SdtHeader, SdtSignature};
use crate::acpi::GENERIC_ADDR_IN_MEM;
use crate::memory::addr::{PhysAddr, VirtAddr};

/// Global HPET
static HPET: Once<Hpet> = Once::new();

pub(super) const HPET_SIG: SdtSignature = *b"HPET";

const HPET_GENERAL_CAP_REG: u32 = 0x00;
const HPET_GENERAL_CONFIG_REG: u32 = 0x10;
const HPET_MAIN_COUNTER_REG: u32 = 0xF0;

const HPET_ENABLE: u64 = 0x0;

const HPET_COUNTER_CLK_PERIOD: u64 = 32;

const MS_IN_FS: u64 = 1_000_000_000_000;

/// The HPET ACPI Table.
#[repr(C, packed)]
pub(super) struct HpetTable {
    _header: SdtHeader,
    _hardware_rev_id: u8,
    _comparator_desc: u8,
    _pci_vendor_id: u16,
    address: GenericAddress,
    _hpet_numer: u8,
    _minimum_tick: u16,
    _oem_attributes: u8,
}

impl HpetTable {
    /// Get the HPET ACPI table present at this address.
    ///
    /// # Safety
    /// `addr` must be a valid pointer to an HPET ACPI table.
    pub unsafe fn new(addr: VirtAddr) -> &'static HpetTable {
        &*addr.as_ptr::<HpetTable>()
    }
}

/// In memory representation of an HPET.
struct Hpet {
    base: VirtAddr,
    freq: u64,
}

impl Hpet {
    /// Initialize an HPET from an ACPI table.
    ///
    /// # Panics
    /// This function will panic in either of the following scenarios:
    /// - The HPET table uses an addressing mode besides system memory
    /// - The HPET's frequency is faster than the maximum of 100 ns.
    pub fn init(table: &HpetTable) -> Self {
        assert!(
            table.address.addr_space_id == GENERIC_ADDR_IN_MEM,
            "ACPI: Unsupported HPET address space"
        );

        let mut this = Self {
            base: PhysAddr::new(table.address.address as usize).as_hddm_virt(),
            freq: 0,
        };

        // Retrieve the HPET's frequency
        // Safety: base is initialized from the ACPI table and guaranteed to be valid. The General
        // Capability Register is a valid register to read from
        let freq = unsafe { this.read(HPET_GENERAL_CAP_REG) } >> HPET_COUNTER_CLK_PERIOD;
        assert!(
            freq <= 0x05f5_e100,
            "ACPI: HPET frequency higher than 100 ns"
        );
        this.freq = freq;

        log::debug!(
            "ACPI: HPET(base = {:x?} freq = {} ns)",
            this.base,
            this.freq
        );

        // Send initialization instructions to the HPET
        // Safety: base is initialized from the ACPI table and guaranteed to be valid. All registers
        // written to are valid HPET registers to write to
        unsafe {
            this.write(HPET_GENERAL_CONFIG_REG, 0 << HPET_ENABLE); // Disable the HPET
            this.write(HPET_MAIN_COUNTER_REG, 0); // Clear the counter
            this.write(HPET_GENERAL_CONFIG_REG, 1 << HPET_ENABLE); // Reenable the HPET
        }

        this
    }

    /// Sleep for `ms` milliseconds.
    fn sleep(&self, ms: u64) {
        // Safety: We're only reading from the main counter
        unsafe {
            let target = self.read(HPET_MAIN_COUNTER_REG) + (ms * MS_IN_FS) / self.freq;
            while self.read(HPET_MAIN_COUNTER_REG) < target {
                core::hint::spin_loop();
            }
        }
    }

    /// Read from an HPET register
    ///
    /// # Safety
    /// `reg` must be a valid HPET register
    unsafe fn read(&self, reg: u32) -> u64 {
        let ptr = self.base.as_ptr::<u64>().byte_offset(reg as isize);
        core::ptr::read_volatile(ptr)
    }

    /// Write to an HPET register
    ///
    /// # Safety
    /// `reg` must be a valid HPET register
    unsafe fn write(&self, reg: u32, data: u64) {
        let ptr = self.base.as_mut_ptr::<u64>().byte_offset(reg as isize);
        core::ptr::write_volatile(ptr, data);
    }
}

/// Sleep for `ms` milliseconds using the global HPET
pub fn sleep(ms: u64) {
    HPET.get()
        .expect("Attempted to sleep with the HPET before it was initialized")
        .sleep(ms);
}

/// Initialize the global HPET from the HPET ACPI table.
///
/// # Panics
/// This function will panic if the global HPET has already been initialized.
pub(super) fn init(table: &HpetTable) {
    HPET.call_once(|| Hpet::init(table));
}
