use raw_cpuid::{CpuId, FeatureInfo};
use spin::{Mutex, MutexGuard, Once};

use crate::{
    arch::io,
    memory::addr::{PhysAddr, VirtAddr},
};

use super::{allocate_vector, handler::interrupt_stack, register_handler};

const SPURIOUS_VECTOR: u32 = 0xFF;

const APIC_BASE_MSR: u32 = 0x1B;
const XAPIC_TPR: u32 = 0xB0;
const XAPIC_SVR: u32 = 0xF0;
const XAPIC_LVT_ERR: u32 = 0x370;
const XAPIC_ESR: u32 = 0x280;

pub static LOCAL_APIC: Once<Mutex<LocalApic>> = Once::new();

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
            log::trace!("{lvt_err_vector}");
            register_handler(lvt_err_vector, lvt_err_handler);

            self.write(XAPIC_LVT_ERR, lvt_err_vector as u32);
        }
    }

    pub fn get_esr(&mut self) -> u32 {
        unsafe {
            self.write(XAPIC_ESR, 0);
            self.read(XAPIC_ESR)
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

pub fn init() -> ApicType {
    let feature_info = CpuId::new()
        .get_feature_info()
        .expect("Able to retrieve CPU feature info");
    let apic_type = ApicType::from(feature_info);

    if apic_type == ApicType::None {
        return apic_type;
    }

    let apic_addr = unsafe { io::rdmsr(APIC_BASE_MSR) };
    let addr = PhysAddr::new(apic_addr as usize).as_hddm_virt();

    let mut lapic = LocalApic::new(addr, apic_type);
    lapic.init();

    LOCAL_APIC.call_once(move || Mutex::new(lapic));

    apic_type
}
