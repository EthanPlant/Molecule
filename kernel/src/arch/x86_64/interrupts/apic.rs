//! Handling for the Advanced Programmable Interrupt Controller (APIC)

use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use raw_cpuid::{CpuId, FeatureInfo};
use spin::Once;

use super::{allocate_vector, idt, InterruptStackFrame};
use crate::acpi::hpet;
use crate::acpi::madt::{IO_APICS, OVERRIDES};
use crate::arch::io;
use crate::memory::addr::{PhysAddr, VirtAddr};
use crate::sync::{Mutex, MutexGuard};

/// APIC base MSR
const IA32_APIC_BASE: u32 = 0x1b;

/// Vector to use for spurious interrupt.
const APIC_SPURIOUS_VECTOR: u32 = 0xff;

/// APIC ID Register
const XAPIC_ID: u32 = 0x020;
/// Task Priority Register (TPR). R/W. Bits 31:8 are reserved.
const XAPIC_TPR: u32 = 0x080;
/// End-Of-Interrupt (EOI) register. Write only. Writing 0 signals the end of an interrupt.
const EOI: u32 = 0x0b0;
/// Spurious Interrupt Vector. R/W. Low byte contains spurious interrupt vector, bit 8 enables APIC.
const XAPIC_SVR: u32 = 0x0f0;
/// Local Vector Table Error. R/W. Contains vector to LVT error interrupt.
const LVT_ERROR: u32 = 0x370;

/// APIC timer interrupt vector R/W.
const APIC_LVT_TIMER: u32 = 0x320;
/// APIC timer initial count. R/W
const APIC_TIMER_INIT_COUNT: u32 = 0x380;
/// Current APIC timer count. Read only.
const APIC_TIMER_COUNT: u32 = 0x390;
/// APIC timer divisor. R/W.
const APIC_TIMER_DIV: u32 = 0x3e0;

/// I/O APIC Version Register
const IOAPIC_VER: u32 = 1;

/// Total count of CPUs in the system
pub static CPU_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Global access to the LAPIC
static LOCAL_APIC: Once<Mutex<LocalApic>> = Once::new();

/// Marks if the bootstrap processor (BSP) is finished initializing and we're ready to initalize the
/// APs
static BSP_READY: AtomicBool = AtomicBool::new(false);

/// The type of APIC used by the CPU
#[derive(Debug)]
enum ApicType {
    /// Legacy APIC
    XApic,
    /// 2nd generation APIC
    X2Apic,
    /// No APIC
    None,
}

impl ApicType {
    /// Return if the APIC type is None, and the CPU doesn't support the APIC
    fn is_none(&self) -> bool {
        matches!(self, ApicType::None)
    }
}

impl From<FeatureInfo> for ApicType {
    fn from(value: FeatureInfo) -> Self {
        if value.has_x2apic() {
            Self::X2Apic
        } else if value.has_apic() {
            Self::XApic
        } else {
            Self::None
        }
    }
}

/// In-memory representation of the CPU's local APIC (LAPIC)
pub struct LocalApic {
    addr: VirtAddr,
    apic_type: ApicType,
}

impl LocalApic {
    /// Create a new LAPIC.
    fn new(addr: VirtAddr, apic_type: ApicType) -> Self {
        Self { addr, apic_type }
    }

    /// Initialize the LAPIC.
    ///
    /// # Panics
    ///
    /// This function panics if the Apic type is [ApicType::None]
    ///
    /// # Safety
    ///
    /// Caller must ensure `self.addr` is a valid LAPIC address.
    pub unsafe fn init(&self) {
        self.write_register(XAPIC_TPR, 0x00); // Clear the TPR to enable all interrupts
        self.set_up_spurious();
        self.set_up_lvt_err();
    }

    /// Calibrate the APIC timer and set it to interrupt on vector `vec`. Returns the calibrated
    /// number of ticks in 10 ms.
    pub fn timer_calibrate(&mut self, vec: u8) -> u32 {
        // Safety: All timer registers are valid registers
        unsafe {
            self.write_register(APIC_TIMER_DIV, 0x03); // Tell APIC timer to use divider 16
            self.write_register(APIC_TIMER_INIT_COUNT, 0xffff_ffff); // Set initial count to -1
            hpet::sleep(10);
            self.write_register(APIC_LVT_TIMER, 1 << 16); // Stop the timer.
            let ticks = 0xffff_ffff - self.read_register(APIC_TIMER_COUNT);

            log::debug!("Calibrated timer ticks {}", ticks);

            self.write_register(APIC_LVT_TIMER, vec as u32 | 0x20000);

            self.write_register(APIC_TIMER_DIV, 1);
            self.write_register(APIC_TIMER_INIT_COUNT, ticks);

            ticks
        }
    }

    /// Send an end-of-interrupt signal to mark an interrupt as acknowledged.
    pub fn eoi(&self) {
        // Safety: LAPIC is valid
        unsafe {
            self.write_register(EOI, 0);
        }
    }

    /// Get the ID of this local APIC
    pub fn get_lapic_id(&self) -> u32 {
        // Safety: LAPIC is valid
        unsafe { self.read_register(XAPIC_ID) }
    }

    /// Read from a LAPIC register.
    ///
    /// # Panics
    ///
    /// This function panics if APIC type is set to [ApicType::None]
    ///
    /// # Safety
    ///
    /// `register` must be a valid LAPIC register.
    unsafe fn read_register(&self, register: u32) -> u32 {
        match self.apic_type {
            ApicType::XApic => {
                let addr = self.addr + register as usize;
                addr.as_mut_ptr::<u32>().read_volatile()
            }
            ApicType::X2Apic => todo!("Implement support for the x2APIC"),
            ApicType::None => unreachable!(),
        }
    }

    /// Write to a LAPIC register.
    ///
    /// # Panics
    ///
    /// This function panics if APIC type is set to [ApicType::None].
    ///
    /// # Safety
    ///
    /// `register` must be a valid LAPIC register.
    unsafe fn write_register(&self, register: u32, value: u32) {
        match self.apic_type {
            ApicType::XApic => {
                let addr = self.addr + register as usize;
                addr.as_mut_ptr::<u32>().write_volatile(value);
            }
            ApicType::X2Apic => todo!("Implement support for the x2APIC"),
            ApicType::None => unreachable!(),
        }
    }

    /// Register the spurious interrupt handler.
    ///
    /// # Safety
    ///
    /// `self.addr` must be a valid LAPIC address.
    unsafe fn set_up_spurious(&self) {
        idt::set_handler(APIC_SPURIOUS_VECTOR as usize, spurious_handler);
        self.write_register(XAPIC_SVR, 0x100 | APIC_SPURIOUS_VECTOR); // Set the SVR to the spurious
                                                                      // interrupt vector and enable
                                                                      // the APIC
    }

    /// Register the LVT error handler.
    ///
    /// # Safety
    ///
    /// `self.addr` must be a valid LAPIC address.
    unsafe fn set_up_lvt_err(&self) {
        let vector = allocate_vector();
        idt::set_handler(vector as usize, lvt_err_handler);
        self.write_register(LVT_ERROR, vector as u32);
    }
}

/// Set up an IRQ using the I/O APIC to point `irq` to interrupt `vec`.
pub fn ioapic_setup_irq(irq: u8, vec: u8) {
    let overrides = OVERRIDES.read();
    for entry in overrides.iter() {
        if entry.irq() == irq {
            ioapic_set_redirect(vec, entry.gsi(), entry.flags());
            return;
        }
    }

    ioapic_set_redirect(vec, irq as u32, 0);
}

/// Set an I/O APIC redirect entry between `gsi` and `vec`.
fn ioapic_set_redirect(vec: u8, gsi: u32, flags: u16) {
    if let Some(ioapic) = ioapic_from_gsi(gsi) {
        let mut redirect_entry: u64 = 0;

        // Active low
        if (flags & (1 << 1)) != 0 {
            redirect_entry |= 1 << 13;
        }

        // Level triggered
        if (flags & (1 << 3)) != 0 {
            redirect_entry |= 1 << 15;
        }

        redirect_entry |= vec as u64;

        let entry = IO_APICS.read()[ioapic];
        let ioredtbl = (gsi - entry.gsi_base()) * 2 + 16; // Get the register for this IRQ

        // Safety: IOREDTBL is a valid I/O APIC register
        unsafe {
            ioapic_write(ioapic, ioredtbl, redirect_entry as u32);
            ioapic_write(ioapic, ioredtbl + 1, redirect_entry as u32);
        }
        log::debug!("Registered redirect (vec={vec}, gsi={gsi})");
    } else {
        log::warn!("Unable to register redirect (vec={vec}, gsi={gsi})");
    }
}

/// Find the I/O APIC for a given GSI, returning the I/O APIC ID.
fn ioapic_from_gsi(gsi: u32) -> Option<usize> {
    let ioapics = IO_APICS.read();

    for (i, entry) in ioapics.iter().enumerate() {
        let max = entry.gsi_base() + ioapic_max_redirect(i);

        // Check if `gsi` is within this I/O APIC's bounds
        if entry.gsi_base() <= gsi || max > gsi {
            return Some(i);
        }
    }

    None
}

/// Get the maximum redirect supported by the given I/O APIC
fn ioapic_max_redirect(ioapic_id: usize) -> u32 {
    // Safety: IOAPIC_VER is a valid I/O APIC register
    unsafe { (ioapic_read(ioapic_id, IOAPIC_VER) & 0x00ff_0000) >> 16 } // Bits 16:23 contain the
                                                                        // max redirection entry
}

/// Read a value from an I/O APIC register.
///
/// # Safety
///
/// The register must be a valid I/O APIC register.
unsafe fn ioapic_read(ioapic_id: usize, register: u32) -> u32 {
    let ioapic = IO_APICS.read()[ioapic_id];
    let ptr: *mut u32 = ioapic.addr().as_mut_ptr();

    // In order to read from an I/O APIC register, we first have to write the register we want to
    // read from
    core::ptr::write_volatile(ptr, register);
    // We then read from IOWIN, which is 16 bytes above the I/O APIC base
    core::ptr::read_volatile(ptr.offset(4))
}

/// Write a value to an I/O APIC register.
///
/// # Safety
///
/// The register must be a valid I/O APIC register.
unsafe fn ioapic_write(ioapic_id: usize, register: u32, data: u32) {
    let ioapic = IO_APICS.read()[ioapic_id];
    let ptr: *mut u32 = ioapic.addr().as_mut_ptr();

    // In order to write an I/O APIC register, we first have to write the register we want to write
    // to
    core::ptr::write_volatile(ptr, register);
    // We then write to IOWIN, which is 16 bytes above the I/O APIC base
    core::ptr::write_volatile(ptr.offset(4), data);
}

/// Get the local APIC.
pub fn get_local_apic() -> MutexGuard<'static, LocalApic> {
    LOCAL_APIC
        .get()
        .expect("Attempted to get LAPIC before it was initialized")
        .lock()
}

/// Get the CPU count.
pub fn get_cpu_count() -> usize {
    CPU_COUNT.load(Ordering::Relaxed)
}

/// Check if the BSP is ready.
pub fn get_bsp_ready() -> bool {
    BSP_READY.load(Ordering::SeqCst)
}

/// Mark the BSP as ready
pub fn set_bsp_ready() {
    BSP_READY.store(true, Ordering::SeqCst);
}

/// Initialize the APIC
pub fn init() {
    let feature_info = CpuId::new()
        .get_feature_info()
        .expect("Unable to retrieve feature info from the CPU");
    let apic_type = ApicType::from(feature_info);

    if apic_type.is_none() {
        panic!("APIC not supported");
    }

    // Safety: IA32_APIC_BASE is the APIC base MSR and always valid to read from
    let apic_base = unsafe { io::rdmsr(IA32_APIC_BASE) };
    let addr = PhysAddr::new((apic_base & 0xffff_0000) as usize).as_hhdm_virt();

    log::debug!("Detected APIC (addr={addr:x}, type={apic_type:?}");
    let lapic = LocalApic::new(addr, apic_type);
    // Safety: `lapic.addr` is a valid LAPIC address returned from the APIC base MSR
    unsafe { lapic.init() };

    LOCAL_APIC.call_once(|| Mutex::new(lapic));
    log::info!("APIC: Local APIC intiialized");
}

extern "x86-interrupt" fn lvt_err_handler(_stack: InterruptStackFrame) {
    log::warn!("LAPIC Error");
}

extern "x86-interrupt" fn spurious_handler(_stack: InterruptStackFrame) {
    log::warn!("Spurious interrupt");
}
