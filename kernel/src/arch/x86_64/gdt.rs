use core::{arch::asm, mem, ptr::addr_of};

use super::PrivilegeLevel;

const GDT_ENTRIES: usize = 3;

pub const KERNEL_CODE_INDEX: u16 = 1;
const KERNEL_DATA_INDEX: u16 = 2;

static mut GDT: [GdtEntry; GDT_ENTRIES] = [
    // Null Descriptor
    GdtEntry::NULL,
    // Kernel Code Descriptor
    GdtEntry::new(
        GdtAccessFlags::PRESENT
            | GdtAccessFlags::KERNEL
            | GdtAccessFlags::SYSTEM
            | GdtAccessFlags::EXECUTABLE
            | GdtAccessFlags::READ_WRITE,
        GdtEntryFlags::LONG_MODE,
    ),
    // Kernel Data Descriptor
    GdtEntry::new(
        GdtAccessFlags::PRESENT
            | GdtAccessFlags::KERNEL
            | GdtAccessFlags::SYSTEM
            | GdtAccessFlags::READ_WRITE,
        GdtEntryFlags::LONG_MODE,
    ),
];

bitflags::bitflags! {
    #[derive(Debug, Copy, Clone)]
    struct GdtEntryFlags: u8 {
        const PROTECTED_MODE = 1 << 6;
        const LONG_MODE = 1 << 5;
    }
}

#[derive(Debug, Copy, Clone)]
struct GdtAccessFlags;

impl GdtAccessFlags {
    const PRESENT: u8 = 1 << 7;
    const KERNEL: u8 = 0 << 5;
    const USER: u8 = 3 << 5;
    const SYSTEM: u8 = 1 << 4;
    const EXECUTABLE: u8 = 1 << 3;
    const DIRECTION: u8 = 1 << 2;
    const READ_WRITE: u8 = 1 << 1;
    const NULL: u8 = 0;
}

#[derive(Debug, Copy, Clone, Default)]
#[repr(transparent)]
pub struct SegmentSelector(u16);

impl SegmentSelector {
    pub const fn new(index: u16, privilege: PrivilegeLevel) -> Self {
        Self(index << 3 | (privilege as u16))
    }

    fn bits(self) -> u16 {
        self.0
    }
}

#[repr(C, packed)]
struct GdtDescriptor {
    size: u16,
    offset: u64,
}

impl GdtDescriptor {
    const fn new(size: u16, offset: u64) -> Self {
        Self { size, offset }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    limit_high_flags: u8,
    base_high: u8,
}

impl GdtEntry {
    const NULL: Self = Self::new(GdtAccessFlags::NULL, GdtEntryFlags::empty());

    const fn new(access: u8, flags: GdtEntryFlags) -> Self {
        Self {
            limit_low: 0x00,
            base_low: 0x00,
            base_middle: 0x00,
            access,
            limit_high_flags: flags.bits() & 0xF0,
            base_high: 0x00,
        }
    }
}

pub fn init() {
    let gdt_descriptor = GdtDescriptor::new(
        (mem::size_of::<[GdtEntry; GDT_ENTRIES]>() - 1) as u16,
        addr_of!(GDT) as u64,
    );

    // Safety: GDT is well defined and the descriptor is valid.
    unsafe {
        load_gdt(&gdt_descriptor);

        set_cs(SegmentSelector::new(
            KERNEL_CODE_INDEX,
            PrivilegeLevel::Kernel,
        ));
        set_ds(SegmentSelector::new(
            KERNEL_DATA_INDEX,
            PrivilegeLevel::Kernel,
        ));
        set_es(SegmentSelector::new(
            KERNEL_DATA_INDEX,
            PrivilegeLevel::Kernel,
        ));
        set_fs(SegmentSelector::new(
            KERNEL_DATA_INDEX,
            PrivilegeLevel::Kernel,
        ));
        set_gs(SegmentSelector::new(
            KERNEL_DATA_INDEX,
            PrivilegeLevel::Kernel,
        ));
        set_ss(SegmentSelector::new(
            KERNEL_DATA_INDEX,
            PrivilegeLevel::Kernel,
        ));
    }

    log::trace!("GDT Entries: {:x?}", unsafe { GDT });
}

unsafe fn load_gdt(descriptor: &GdtDescriptor) {
    asm!("lgdt [{}]", in(reg) descriptor, options(nostack));
}

#[allow(binary_asm_labels)]
unsafe fn set_cs(selector: SegmentSelector) {
    asm!(
        "push {selector}",
        "lea {tmp}, [rip + 1f]",
        "push {tmp}",
        "retfq",
        "1:",
        selector = in(reg) u64::from(selector.bits()),
        tmp = lateout(reg) _,
    );
}

unsafe fn set_ds(selector: SegmentSelector) {
    asm!("mov ds, {0:x}", in(reg) selector.bits());
}

unsafe fn set_es(selector: SegmentSelector) {
    asm!("mov es, {0:x}", in(reg) selector.bits());
}

unsafe fn set_fs(selector: SegmentSelector) {
    asm!("mov fs, {0:x}", in(reg) selector.bits());
}

unsafe fn set_gs(selector: SegmentSelector) {
    asm!("mov gs, {0:x}", in(reg) selector.bits());
}

unsafe fn set_ss(selector: SegmentSelector) {
    asm!("mov ss, {0:x}", in(reg) selector.bits());
}
