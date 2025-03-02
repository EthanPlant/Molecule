use spin::{Mutex, Once};

use crate::memory::addr::{PhysAddr, VirtAddr};

use super::{SdtHeader, SdtSignature};

pub const HPET_SIG: SdtSignature = SdtSignature(*b"HPET");

static HPET: Once<Hpet> = Once::new();

const HPET_GENERAL_CAP_REG: u32 = 0x0;
const HPET_COUNTER_CLK_PERIOD: u64 = 32;

const HPET_GENERAL_CONFIG: u32 = 0x10;
const HPET_ENABLE_CNF: u64 = 0;

const HPET_MAIN_COUNTER: u32 = 0xF0;

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
struct HpetAddr {
    addr_space_width: u8,
    register_bit_width: u8,
    register_bit_offset: u8,
    reserved: u8,
    address: u64,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct HpetTable {
    header: SdtHeader,
    hardware_rev_id: u8,
    comparator_desc: u8,
    pci_vendor_id: u16,
    address: HpetAddr,
    hpet_number: u8,
    minimum_tick: u16,
    oem_attributes: u8,
}

impl HpetTable {
    pub fn new(addr: VirtAddr) -> Self {
        unsafe { *(usize::from(addr) as *const HpetTable) }
    }
}

#[derive(Debug)]
pub struct Hpet {
    base: VirtAddr,
    freq: u64,
}

impl Hpet {
    pub fn init(table: &HpetTable) -> Self {
        assert!(
            table.address.addr_space_width == 0,
            "Unsupported HPET address space"
        );
        let mut hpet = Self {
            base: PhysAddr::new(table.address.address as usize).as_hddm_virt(),
            freq: 0,
        };

        let freq = unsafe { hpet.read_reg(HPET_GENERAL_CAP_REG) } >> HPET_COUNTER_CLK_PERIOD;
        assert!(freq <= 0x05F5_E100, "HPET frequency too high");
        hpet.freq = freq;

        unsafe {
            hpet.write_reg(HPET_GENERAL_CONFIG, 0 << HPET_ENABLE_CNF);
            hpet.write_reg(HPET_MAIN_COUNTER, 0);
            hpet.write_reg(HPET_GENERAL_CONFIG, 1 << HPET_ENABLE_CNF);
        }

        hpet
    }

    fn sleep(&self, ms: u64) {
        let target =
            unsafe { self.read_reg(HPET_MAIN_COUNTER) } + (ms * 1_000_000_000_000) / self.freq;
        while unsafe { self.read_reg(HPET_MAIN_COUNTER) } < target {
            core::hint::spin_loop();
        }
    }

    unsafe fn read_reg(&self, reg: u32) -> u64 {
        let ptr = (usize::from(self.base) + reg as usize) as *const u64;
        core::ptr::read_volatile(ptr)
    }

    unsafe fn write_reg(&self, reg: u32, val: u64) {
        let ptr = (usize::from(self.base) + reg as usize) as *mut u64;
        core::ptr::write_volatile(ptr, val);
    }
}

pub fn init_hpet(table: &HpetTable) {
    HPET.call_once(|| Hpet::init(table));
}

pub fn hpet_sleep(ms: u64) {
    HPET.get().expect("HPET is initialized").sleep(ms);
}
