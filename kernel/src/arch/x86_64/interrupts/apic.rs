use core::{
    f64::MAX_EXP,
    ops::{Add, AddAssign},
    ptr,
    sync::atomic::{AtomicBool, AtomicU64, AtomicU8, AtomicUsize, Ordering},
};

use raw_cpuid::{CpuId, FeatureInfo};
use spin::{Lazy, Once};

use crate::{
    acpi::{
        hpet::hpet_sleep,
        madt::{MadtEntry, IO_APICS, REDIRECTS},
        ACPI_TABLES,
    },
    arch::io,
    drivers::framebuffer::console::print,
    memory::addr::{PhysAddr, VirtAddr},
    sync::{Mutex, MutexGuard},
};

use super::{allocate_vector, disable_pic, handler::interrupt_stack, register_handler};

const SPURIOUS_VECTOR: u32 = 0xFF;

const APIC_BASE_MSR: u32 = 0x1B;
const XAPIC_ID: u32 = 0x020;
const XAPIC_TPR: u32 = 0x080;
const XAPIC_SVR: u32 = 0xF0;
const XAPIC_LVT_ERR: u32 = 0x370;
const XAPIC_ESR: u32 = 0x280;
const XAPIC_EOI: u32 = 0x0B0;
const APIC_LVT_TIMER: u32 = 0x320;
const APIC_TIMER_INIT: u32 = 0x380;
const APIC_TIMER_CURRENT: u32 = 0x390;
const APIC_TIMER_DIV: u32 = 0x3E0;

pub static CPU_COUNT: AtomicUsize = AtomicUsize::new(0);

pub static LOCAL_APIC: Once<Mutex<LocalApic>> = Once::new();
static BSP_APIC_ID: AtomicU64 = AtomicU64::new(0);

pub static TICKS: Lazy<Mutex<u64>> = Lazy::new(|| Mutex::new(0));
static TIMER_VEC: AtomicU8 = AtomicU8::new(0);

static BSP_READY: AtomicBool = AtomicBool::new(false);

pub struct LocalApic {
    addr: VirtAddr,
    apic_type: ApicType,
}

interrupt_stack!(lvt_err_handler, |_stack| {
    log::error!("Lapic Error!");
    log::error!("ESR={:#x}", get_local_apic().get_esr());
});

impl LocalApic {
    fn new(addr: VirtAddr, apic_type: ApicType) -> Self {
        Self { addr, apic_type }
    }

    fn init(&mut self) {
        unsafe {
            self.write(XAPIC_TPR, 0x00);
            self.write(XAPIC_SVR, 0x100 | SPURIOUS_VECTOR);
            let lvt_err_vector = allocate_vector();
            register_handler(lvt_err_vector, lvt_err_handler);

            self.write(XAPIC_LVT_ERR, lvt_err_vector as u32);

            self.timer_calibrate();
        }
    }

    pub fn timer_calibrate(&mut self) {
        if TIMER_VEC.load(Ordering::Relaxed) == 0 {
            let vec = allocate_vector();
            register_handler(vec, timer_handler);
            ioapic_setup_irq(0, vec, 0);
            TIMER_VEC.store(vec, Ordering::Relaxed);
        }

        unsafe {
            self.write(APIC_TIMER_DIV, 0x1);
            self.write(APIC_TIMER_INIT, 0xFFFF_FFFF);
            hpet_sleep(10);
            self.write(APIC_LVT_TIMER, (1 << 16) | 0xFF);
            let ticks = 0xFFFF_FFFF - self.read(APIC_TIMER_CURRENT);

            log::debug!("Calibrated timer ticks: {}", ticks);

            self.write(
                APIC_LVT_TIMER,
                TIMER_VEC.load(Ordering::Relaxed) as u32 | 0x20000,
            );
            self.write(APIC_TIMER_DIV, 0x1);
            self.write(APIC_TIMER_INIT, ticks);
        }
    }

    pub fn get_esr(&mut self) -> u32 {
        unsafe {
            self.write(XAPIC_ESR, 0);
            self.read(XAPIC_ESR)
        }
    }

    fn bsp_id(&self) -> u32 {
        unsafe { self.read(XAPIC_ID) }
    }

    #[inline]
    pub fn eoi(&mut self) {
        unsafe {
            self.write(XAPIC_EOI, 0);
        }
    }

    fn register_to_xapic_addr(&self, register: u32) -> VirtAddr {
        self.addr + register as usize
    }

    unsafe fn write(&mut self, register: u32, value: u32) {
        if self.apic_type == ApicType::Xapic {
            let addr = self.register_to_xapic_addr(register);
            addr.as_mut_ptr::<u32>().write_volatile(value);
        } else {
            unreachable!("Unsupported APIC type");
        }
    }

    unsafe fn read(&self, register: u32) -> u32 {
        if self.apic_type == ApicType::Xapic {
            let addr = self.register_to_xapic_addr(register);
            addr.as_ptr::<u32>().read_volatile()
        } else {
            unreachable!("Unsupported APIC type");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApicType {
    Xapic,
    X2apic,
    None,
}

impl From<FeatureInfo> for ApicType {
    fn from(value: FeatureInfo) -> Self {
        if value.has_x2apic() {
            Self::X2apic
        } else if value.has_apic() {
            Self::Xapic
        } else {
            Self::None
        }
    }
}

pub fn get_local_apic() -> MutexGuard<'static, LocalApic> {
    LOCAL_APIC.get().expect("Lapic is initialized").lock()
}

pub fn get_cpu_count() -> usize {
    CPU_COUNT.load(Ordering::Relaxed)
}

pub fn get_bsp_id() -> u64 {
    BSP_APIC_ID.load(Ordering::SeqCst)
}

pub fn get_bsp_ready() -> bool {
    BSP_READY.load(Ordering::SeqCst)
}

pub fn set_bsp_ready() {
    BSP_READY.store(true, Ordering::SeqCst);
}

unsafe fn ioapic_read(ioapic_id: usize, register: u32) -> u32 {
    let ioapic = IO_APICS.read()[ioapic_id];
    let addr = ioapic.addr();
    let ptr: *mut u32 = addr.as_mut_ptr();

    ptr::write_volatile(ptr, register);
    ptr::read(ptr.offset(4))
}

unsafe fn ioapic_write(ioapic_id: usize, register: u32, data: u32) {
    let ioapic = IO_APICS.read()[ioapic_id];
    let addr = ioapic.addr();
    let ptr: *mut u32 = addr.as_mut_ptr();

    ptr::write_volatile(ptr, register);
    ptr::write_volatile(ptr.offset(4), data);
}

fn ioapic_max_redirect(ioapic_id: usize) -> u32 {
    unsafe { (ioapic_read(ioapic_id, 1) & 0x00FF_0000) >> 16 }
}

fn ioapic_from_redirect(gsi: u32) -> Option<usize> {
    let ioapics = IO_APICS.read();

    for (i, entry) in ioapics.iter().enumerate() {
        let max_redirect = entry.interrupt_base() + ioapic_max_redirect(i) > gsi;

        if entry.interrupt_base() <= gsi || max_redirect {
            return Some(i);
        }
    }

    None
}

fn ioapic_set_redirect(vec: u8, gsi: u32, flags: u16, status: i32) {
    if let Some(ioapic) = ioapic_from_redirect(gsi) {
        let mut redirect = 0;

        if (flags & (1 << 1)) != 0 {
            redirect |= (1 << 13) as u8;
        }

        if (flags & (1 << 3)) != 0 {
            redirect |= (1 << 15) as u8;
        }

        if status == 1 {
            redirect |= (1 << 16) as u8;
        }

        redirect |= vec;
        redirect |= (0usize << 56) as u8; // TODO: Properly set destination mode

        let entry = IO_APICS.read()[ioapic];
        let ioredtbl = (gsi - entry.interrupt_base()) * 2 + 16;

        unsafe {
            ioapic_write(ioapic, ioredtbl, redirect as _);
            ioapic_write(ioapic, ioredtbl + 1, (redirect as u64 >> 32) as _);
        }
        log::debug!("Registered redirect (vec={vec}, gsi={gsi})");
    } else {
        log::warn!("Unable to register redirect (vec={vec}, gsi={gsi})");
    }
}

pub fn ioapic_setup_irq(irq: u8, vec: u8, status: i32) {
    let overrides = REDIRECTS.read();

    for entry in overrides.iter() {
        if entry.irq() == irq {
            ioapic_set_redirect(vec, entry.system_int(), entry.flags(), status);
            return;
        }
    }

    ioapic_set_redirect(vec, irq as u32, 0, status);
}

pub fn init() {
    let feature_info = CpuId::new()
        .get_feature_info()
        .expect("Able to retrieve CPU feature info");
    let apic_type = ApicType::from(feature_info);

    if apic_type == ApicType::None {
        return;
    }

    let apic_addr = unsafe { io::rdmsr(APIC_BASE_MSR) };
    let addr = PhysAddr::new((apic_addr & 0xFFFF_0000) as usize).as_hddm_virt();

    log::debug!("Detected APIC (addr={addr:x?}, type={apic_type:?})");

    let mut lapic = LocalApic::new(addr, apic_type);
    lapic.init();

    let bsp_id = lapic.bsp_id();
    BSP_APIC_ID.store(bsp_id as u64, Ordering::SeqCst);

    disable_pic();

    LOCAL_APIC.call_once(move || Mutex::new(lapic));
}

pub fn init_ap() {
    get_local_apic().init();
}

interrupt_stack!(timer_handler, |_stack| {
    let mut ticks = TICKS.lock();
    *ticks += 1;
    get_local_apic().eoi();
});
