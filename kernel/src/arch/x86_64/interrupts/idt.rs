use core::{arch::asm, ptr::addr_of};

use spin::Mutex;

use crate::arch::{
    x86_64::gdt::{SegmentSelector, KERNEL_CODE_INDEX},
    PrivilegeLevel,
};

use super::handler::HandlerFunc;

const IDT_ENTRIES: usize = 256;

/// The Interrupt Descriptor Table (IDT) entries.
pub static IDT: Mutex<Idt> = Mutex::new(Idt::new());

/// An Interrupt Descriptor Table (IDT)
#[repr(C, align(16))]
pub struct Idt {
    /// The entries of the IDT.
    pub entries: [IdtEntry; IDT_ENTRIES],
}

impl Idt {
    /// Create a new empty IDT
    const fn new() -> Self {
        Self {
            entries: [IdtEntry::EMPTY; IDT_ENTRIES],
        }
    }
}

/// The Interrupt Descriptor Table (IDT) descriptor. This structure will get loaded into the CPU through the `lidt` instruction.
/// to tell the CPU where the IDT is located in memory.
#[derive(Debug)]
#[repr(C, packed)]
struct IdtDescriptor {
    /// The size of the IDT in bytes minus 1.
    ///
    /// In theory this should always be 4095 bytes. As the IDT typically contains 256 entries, each 16 bytes in size.
    /// If more than 256 entries are present, they will be ignored by the CPU. While an IDT with less than 256 entries is valid,
    /// any attempt to access an invalid entry will result in a General Protection Fault.
    size: u16,
    /// The address of the IDT in memory. This should be a pointer to the beginning of an array of [`IdtEntries`](IdtEntry).
    offset: u64,
}

impl IdtDescriptor {
    /// Create a new IDT descriptor.
    fn new(size: u16, offset: u64) -> Self {
        IdtDescriptor { size, offset }
    }
}

/// Represents the type of interrupt that occurs. This data will be stored in an [IdtEntry](IdtEntry) to describe the type of interrupt.
///
/// Almost all of the time, an interrupt will have the `Interrupt` variant as its type. Only use the `Trap` variant if absolutely required.
#[derive(Debug)]
enum GateType {
    /// An interrupt gate, entries with this type will disable interrupts while the handler is running.
    Interrupt = 0x0E,
    /// A trap gate, entries with this type will not disable interrupts while the handler is running.
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

/// The attributes of an IDT entry. This structure contains information about the privilege levels allowed to call this interrupt via the `INT` instruction,
/// and the [`GateType`] of the entry.
#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
struct IdtEntryAttributes(u8);

#[allow(dead_code)]
impl IdtEntryAttributes {
    /// Create a new IDT entry attributes structure with the corresponding [`PrivilegeLevel`] and [`GateType`].
    const fn new(privilege: PrivilegeLevel, gate_type: GateType) -> Self {
        IdtEntryAttributes(1 << 7 | (privilege as u8) << 5 | gate_type as u8)
    }

    /// Create a new IDT entry attributes structure with default values.
    /// This will create an interrupt gate with kernel privilege level.
    // Can't derive Default because this needs to be const
    const fn default() -> Self {
        Self::new(PrivilegeLevel::Kernel, GateType::Interrupt)
    }

    /// Get the [`GateType`] of the IDT entry.
    fn gate_type(self) -> GateType {
        (self.0 & 0x0F).into()
    }

    /// Get the [`PrivilegeLevel`] of the IDT entry.
    fn privilege(self) -> PrivilegeLevel {
        ((self.0 >> 5) & 0x03).into()
    }
}

/// In-memory representation of an IDT entry.
/// The IDT is an array of these structures, each representing a single interrupt or exception.
#[derive(Debug)]
#[repr(C)]
pub struct IdtEntry {
    /// The lower 16 bits of the address of the interrupt handler function.
    offset_low: u16,
    /// The segment selector that the CPU will load when the interrupt handler is called.
    selector: SegmentSelector,
    /// The Interrupt Stack Table index to use for this interrupt. This field is currently unused and should be set to 0.
    ist: u8,
    /// The attributes of the IDT entry
    attributes: IdtEntryAttributes,
    /// The middle 16 bits of the address of the interrupt handler function.
    offset_middle: u16,
    /// The upper 32 bits of the address of the interrupt handler function.
    offset_high: u32,
    /// Reserved field, must be set to 0.
    reserved: u32,
}

impl IdtEntry {
    /// An empty IDT entry used as a placeholder.
    /// This entry has a null function pointer, a kernel code segment selector, and default attributes.
    /// Any attempt to call this entry will result in a Page Fault, as the function pointer is set to 0.
    const EMPTY: Self = {
        // Safety: Segment selector and attributes are valid, we're using 0 as a placeholder function offset.
        // While this will lead to a Page Fault Exception if this interrupt is ever called, it is guaranteed to be well-defined behvaiour.
        unsafe {
            Self::new(
                0,
                SegmentSelector::new(KERNEL_CODE_INDEX, PrivilegeLevel::Kernel),
                IdtEntryAttributes::default(),
            )
        }
    };

    /// Create a new IDT entry, with a given offset, segment selector, and attributes.
    ///
    /// # Safety
    /// `offset` **must** point to a valid interrupt [handler function](HandlerFunc) to avoid unedefined behaviour.
    const unsafe fn new(
        offset: usize,
        selector: SegmentSelector,
        attributes: IdtEntryAttributes,
    ) -> Self {
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

    /// Set the function pointer of the IDT entry to a given function.
    ///
    /// # Safety
    /// This function must be a valid interrupt [handler](HandlerFunc), and cannot be any generic function.
    pub unsafe fn set_func(&mut self, func: HandlerFunc) {
        let func_ptr = func as usize;

        self.offset_low = func_ptr as u16;
        self.offset_middle = (func_ptr >> 16) as u16;
        self.offset_high = (func_ptr >> 32) as u32;
    }
}

/// Initialize the IDT and load it into the CPU.
pub fn init() {
    let idt_descriptor = IdtDescriptor::new(
        (core::mem::size_of::<[IdtEntry; IDT_ENTRIES]>() - 1) as u16,
        addr_of!(IDT.lock().entries).addr() as u64,
    );

    log::trace!("{:x?}", idt_descriptor);

    // Safety: IDT is well defined and the descriptor is valid.
    unsafe {
        load_idt(&idt_descriptor);
    }
}

/// Load a [`IdtDescriptor`] into the CPU.
///
/// # Safety
/// `descriptor` must point to a valid IDT descriptor.
unsafe fn load_idt(descriptor: &IdtDescriptor) {
    asm!(
        "lidt [{}]",
        in(reg) descriptor,
        options(nostack),
    );
}
