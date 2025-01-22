use core::{arch::asm, ptr::addr_of};

use crate::arch::{x86_64::gdt::{SegmentSelector, KERNEL_CODE_INDEX}, PrivilegeLevel};

const IDT_ENTRIES: usize = 256;

pub static mut IDT: [IdtEntry; IDT_ENTRIES] = [IdtEntry::EMPTY; IDT_ENTRIES];

#[derive(Debug)]
#[repr(C, packed)]
struct IdtDescriptor {
    size: u16,
    offset: u64,
}

impl IdtDescriptor {
    fn new(size: u16, offset: u64) -> Self {
        IdtDescriptor { size, offset }
    }
}

#[derive(Debug)]
enum GateType {
    Interrupt = 0x0E,
    Trap = 0x0F,
}

impl From<u8> for GateType {
    fn from(value: u8) -> Self {
        match value {
            0x0E => GateType::Interrupt,
            0x0F => GateType::Trap,
            _ => unreachable!("Invalid gate type"),
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct IdtEntryAttributes(u8);

impl IdtEntryAttributes {
    const fn new(privilege: PrivilegeLevel, gate_type: GateType) -> Self {
        IdtEntryAttributes(1 << 7 | (privilege as u8) << 5 | gate_type as u8)
    }

    // Can't derive Default because this needs to be const
    const fn default() -> Self {
        Self::new(PrivilegeLevel::Kernel, GateType::Interrupt)
    }

    fn gate_type(self) -> GateType {
        (self.0 & 0x0F).into()
    }

    fn privilege(self) -> PrivilegeLevel {
        ((self.0 >> 5) & 0x03).into()
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct IdtEntry {
    offset_low: u16,
    selector: SegmentSelector,
    ist: u8,
    attributes: IdtEntryAttributes,
    offset_middle: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    const EMPTY: Self = Self::new(0, SegmentSelector::new(KERNEL_CODE_INDEX, PrivilegeLevel::Kernel), IdtEntryAttributes::default());

    const fn new(offset: u64, selector: SegmentSelector, attributes: IdtEntryAttributes) -> Self {
        IdtEntry {
            offset_low: offset as u16,
            selector,
            ist: 0,
            attributes,
            offset_middle: (offset >> 16) as u16,
            offset_high: (offset >> 32) as u32,
            reserved: 0,
        }
    }
}

pub fn init() {
    let idt_descriptor = IdtDescriptor::new(
        (core::mem::size_of::<[IdtEntry; IDT_ENTRIES]>() - 1) as u16,
        addr_of!(IDT).addr() as u64,
    );

    log::trace!("{:x?}", idt_descriptor);

    // Safety: IDT is well defined and the descriptor is valid.
    unsafe {
        load_idt(&idt_descriptor);
    }
}

unsafe fn load_idt(descriptor: &IdtDescriptor) {
        asm!(
            "lidt [{}]",
            in(reg) descriptor,
            options(nostack),
        );
}
