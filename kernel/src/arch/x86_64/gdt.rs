//! Global Descriptor Table (GDT) management.

use core::{arch::asm, mem, ptr::addr_of};

use super::PrivilegeLevel;

const GDT_ENTRIES: usize = 3;

pub const KERNEL_CODE_INDEX: u16 = 1;
const KERNEL_DATA_INDEX: u16 = 2;

/// The Global Descriptor Table (GDT) entries.
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
    /// Flags for GDT entries.
    struct GdtEntryFlags: u8 {
        /// Indicates the entry is a 32-bit entry.
        const PROTECTED_MODE = 1 << 6;
        /// Indicates the entry is a 64-bit entry.
        const LONG_MODE = 1 << 5;
    }
}

/// Flags present in the access byte of a GDT entry.
#[derive(Debug, Copy, Clone)]
struct GdtAccessFlags;

impl GdtAccessFlags {
    /// The present flag: This is always 1 for a valid entry.
    const PRESENT: u8 = 1 << 7;
    /// Kernel privilege flag: This indicates the segment is a kernel segment.
    const KERNEL: u8 = 0 << 5;
    /// User privilege flag: This indicates the segment is a user segment.
    const USER: u8 = 3 << 5;
    /// System flag: If present, this is a code or data segment, if absent this is a system segment.
    const SYSTEM: u8 = 1 << 4;
    /// Executable flag: If present, this flag is a code segment which can be executed from, if absent this is a data segment.
    const EXECUTABLE: u8 = 1 << 3;
    /// Direction flag:
    /// If the segment is a data segment, this flag indicates the direction of growth of the segment. If present the segment grows downwards, if absent the segment grows upwards.
    /// If the segment is a code segment, this flag acts as the conforming bit. If present, this segment can be executed from a lower privilege level, if absent this segment can only be executed from the same privilege level.
    const DIRECTION: u8 = 1 << 2;
    /// The read/write flag:
    /// If the segment is a data segment, this flag indicates whether or not the segment is writeable, read access is always allowed.
    /// If the segment is a code segment, this flag indicates whether or not read access is allowed. Write access is never allowed.
    const READ_WRITE: u8 = 1 << 1;
    /// Helper flag to indicate a null descriptor.
    const NULL: u8 = 0;
}

/// A segment selector is a reference to a segment in the GDT, used to load a segment register.
/// Internally this is represented as a 16-bit value, where the bottom two bits represent the privilege level of the segment,
/// and the remaining bits represent the index of the segment in the GDT.
#[derive(Debug, Copy, Clone, Default)]
#[repr(transparent)]
pub struct SegmentSelector(u16);

impl SegmentSelector {
    /// Creates a new segment selector given a GDT index and privilege level.
    pub const fn new(index: u16, privilege: PrivilegeLevel) -> Self {
        Self(index << 3 | (privilege as u16))
    }

    /// Returns the raw 16-bit value of the segment selector.
    fn bits(self) -> u16 {
        self.0
    }
}

/// The GDT descriptor contains the size and location of the GDT in memory. This is used to load the GDT into the CPU with the `lgdt` instruction.
#[repr(C, packed)]
struct GdtDescriptor {
    /// The size of the GDT inb bytes, subtracted by 1.
    /// The GDT can have a maximum size of 65536 bytes, or 8192 entries. Furthermore, the size must be larger than 0.
    size: u16,
    /// The address of the GDT in memory. This must be a continuous block of [`GdtEntry`] entries, and the first entry must be the [null descriptor](GdtEntry::NULL).
    offset: u64,
}

impl GdtDescriptor {
    /// Create a new GDT descriptor with the given size and offset.
    const fn new(size: u16, offset: u64) -> Self {
        Self { size, offset }
    }
}

/// A singular GDT entry. This is a 64-bit structure which represents a single segment in the GDT.
/// This structure contains information about the segment, such as the base address, limit, access flags, and flags.
/// On ``x86_64``, the base and limit are ignored, and each segment covers the entire address space, but they are still necessary for
/// compatibility with the x86 architecture.
#[derive(Debug, Copy, Clone)]
#[repr(C)]
struct GdtEntry {
    /// The bottom 16 bits of the segment limit.
    limit_low: u16,
    /// The bottom 16 bits of the base address.
    base_low: u16,
    /// The middle 8 bits of the base address.
    base_middle: u8,
    /// The access flags for the segment.
    access: u8,
    /// The lower four bits represent the upper four bits of the limit. The upper four bits represent flags.
    limit_high_flags: u8,
    /// The upper 8 bits of the base address.
    base_high: u8,
}

impl GdtEntry {
    /// A null GDT entry with all fields set to 0. All GDT's must contain a null descriptor as their first entry.
    const NULL: Self = Self::new(GdtAccessFlags::NULL, GdtEntryFlags::empty());

    /// Create a new GDT entry with the given access flags and flags.
    ///
    /// # Example:
    /// ```
    /// GdtEntry::new(
    ///     GdtAccessFlags::PRESENT
    ///     | GdtAccessFlags::KERNEL
    ///     | GdtAccessFlags::SYSTEM
    ///     | GdtAccessFlags::EXECUTABLE
    ///     | GdtAccessFlags::READ_WRITE,
    ///     GdtEntryFlags::LONG_MODE,
    ///);
    /// ```
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

/// Initializes the GDT, loading the values in [`GDT`] into the CPU.
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

/// Loads the GDT into the CPU.
///
/// # Safety
/// `descriptor` must point to a valid GDT.
unsafe fn load_gdt(descriptor: &GdtDescriptor) {
    asm!("lgdt [{}]", in(reg) descriptor, options(nostack));
}

/// Sets the code segment register to the given segment selector by executing a far return.
///
/// # Safety
/// `selector` must be a valid segment selector, pointing to an existing GDT entry with the correct privilege level.
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

/// Sets the data segment register to the given segment selector.
///
/// # Safety
/// `selector` must be a valid segment selector, pointing to an existing GDT entry with the correct privilege level.
unsafe fn set_ds(selector: SegmentSelector) {
    asm!("mov ds, {0:x}", in(reg) selector.bits());
}

/// Sets the extra segment register to the given segment selector.
///
/// # Safety
/// `selector` must be a valid segment selector, pointing to an existing GDT entry with the correct privilege level.
unsafe fn set_es(selector: SegmentSelector) {
    asm!("mov es, {0:x}", in(reg) selector.bits());
}

/// Sets the fs segment register to the given segment selector.
///
/// # Safety
/// `selector` must be a valid segment selector, pointing to an existing GDT entry with the correct privilege level.
unsafe fn set_fs(selector: SegmentSelector) {
    asm!("mov fs, {0:x}", in(reg) selector.bits());
}

/// Sets the gs segment register to the given segment selector.
///
/// # Safety
/// `selector` must be a valid segment selector, pointing to an existing GDT entry with the correct privilege level.
unsafe fn set_gs(selector: SegmentSelector) {
    asm!("mov gs, {0:x}", in(reg) selector.bits());
}

/// Sets the stack segment register to the given segment selector.
///
/// # Safety
/// `selector` must be a valid segment selector, pointing to an existing GDT entry with the correct privilege level.
unsafe fn set_ss(selector: SegmentSelector) {
    asm!("mov ss, {0:x}", in(reg) selector.bits());
}
