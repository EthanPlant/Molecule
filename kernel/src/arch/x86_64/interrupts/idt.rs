//! Interrupt Descriptor Table (IDT)

use core::arch::asm;
use core::mem;
use core::ptr::addr_of;

use spin::RwLock;

use super::InterruptStackFrame;
use crate::arch::x86_64::gdt::{SegmentSelector, GDT_KERNEL_CODE};
use crate::arch::x86_64::interrupts::exception::register_exceptions;
use crate::arch::x86_64::PrivilegeLevel;

const IDT_ENTRIES: usize = 256;

/// Global access to the IDT.
pub static IDT: RwLock<Idt> = RwLock::new(Idt::new());

type Handler = extern "x86-interrupt" fn(_: InterruptStackFrame);
type HandlerWithErrorCode = extern "x86-interrupt" fn(_: InterruptStackFrame, _: u64);

/// The Interrupt Descriptor table is a list of 256 [IdtEntry], each pointing to a handler function
/// to run when an interrupt occurs.
pub struct Idt {
    entries: [IdtEntry; IDT_ENTRIES],
}

impl Idt {
    const fn new() -> Self {
        Self {
            entries: [IdtEntry::EMPTY; IDT_ENTRIES],
        }
    }

    /// Set the handler for the given interrupt.
    pub fn set_handler(&mut self, index: usize, handler: Handler) {
        self.entries[index].set_handler(handler);
    }

    /// Set the handler that accepts an error code for the given interrupt.
    pub fn set_handler_with_error_code(&mut self, index: usize, handler: HandlerWithErrorCode) {
        self.entries[index].set_handler_with_error_code(handler);
    }
}

/// Represents the type of interrupt that occurs. This data will be stored in an
/// [IdtEntry] to describe the type of interrupt.
///
/// Almost all of the time, an interrupt will have the `Interrupt` variant as its type. Only use the
/// `Trap` variant if absolutely required.
enum GateType {
    Interrupt = 0x0e,
    Trap = 0x0f,
}

impl From<u8> for GateType {
    fn from(value: u8) -> Self {
        match value {
            0x0e => Self::Interrupt,
            0x0f => Self::Trap,
            _ => unreachable!("Invalid interrupt type"),
        }
    }
}

const IDT_ENTRY_PRESENT: u8 = 1 << 7;
/// An IDT entry's attributes, containing its privilege level and gate type.
#[repr(transparent)]
struct IdtEntryAttributes(u8);

impl IdtEntryAttributes {
    /// Construct the IDT entry attributes for a given privilege level and gate type.
    const fn new(privilege: PrivilegeLevel, gate_type: GateType) -> Self {
        IdtEntryAttributes(IDT_ENTRY_PRESENT | (privilege as u8) << 5 | gate_type as u8)
    }

    /// Construct a set of default values, with a [PrivilegeLevel::Kernel] and
    /// [GateType::Interrupt].
    const fn default() -> Self {
        Self::new(PrivilegeLevel::Kernel, GateType::Interrupt)
    }
}

/// In memory representation of an IDT entry.
#[repr(C, packed)]
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
    /// An empty IDT entry, with default attributes and a NULL handler
    pub const EMPTY: Self = Self::new_from_offset(
        0,
        SegmentSelector::new(GDT_KERNEL_CODE, PrivilegeLevel::Kernel),
        IdtEntryAttributes::default(),
    );

    /// Construct a new IDT entry with a given offset.
    const fn new_from_offset(
        offset: usize,
        selector: SegmentSelector,
        attributes: IdtEntryAttributes,
    ) -> Self {
        Self {
            offset_low: offset as u16,
            selector,
            ist: 0,
            attributes,
            offset_middle: (offset >> 16) as u16,
            offset_high: (offset >> 32) as u32,
            reserved: 0,
        }
    }

    /// Set the IDT entry's handler to a given function.
    pub fn set_handler(&mut self, handler: Handler) {
        let ptr = handler as usize;

        self.offset_low = ptr as u16;
        self.offset_middle = (ptr >> 16) as u16;
        self.offset_high = (ptr >> 32) as u32;
    }

    /// Set the IDT entry's handler to a given function that accepts an error code.
    pub fn set_handler_with_error_code(&mut self, handler: HandlerWithErrorCode) {
        let ptr = handler as usize;

        self.offset_low = ptr as u16;
        self.offset_middle = (ptr >> 16) as u16;
        self.offset_high = (ptr >> 32) as u32;
    }
}

/// The IDT descriptor contains the address and size of an [IDT](Idt). This structure is loaded into
/// the CPU through the `lidt` instruction.
#[repr(C, packed)]
struct IdtDescriptor {
    size: u16,
    offset: u64,
}

impl IdtDescriptor {
    fn new(size: u16, offset: u64) -> Self {
        Self { size, offset }
    }
}

/// Initialize and load the IDT.
pub fn init() {
    log::info!("Interrupts: Initializing IDT");
    let idt_descriptor = IdtDescriptor::new(
        (mem::size_of::<[IdtEntry; IDT_ENTRIES]>() - 1) as u16,
        addr_of!(IDT.read().entries) as u64,
    );

    // Safety: IDT descriptor is valid and points to the IDT
    unsafe {
        load_idt(&idt_descriptor);
    }

    log::info!(
        "Interrupts: IDT loaded at {:x?}",
        addr_of!(IDT.read().entries)
    );

    register_exceptions();
    log::info!("Interrupts: Registered CPU exceptions");
}

/// Set the handler for a given interrupt
pub fn set_handler(vec: usize, handler: Handler) {
    log::debug!("Set interrupt {vec} to {handler:?}");
    IDT.write().set_handler(vec, handler);
}

/// Load the idt pointed to by `descriptor` into the CPU.
///
/// # Safety
/// `descriptor` must point to a valid IDT descriptor.
unsafe fn load_idt(descriptor: &IdtDescriptor) {
    asm!("lidt [{}]", in(reg) descriptor, options(nostack));
}
