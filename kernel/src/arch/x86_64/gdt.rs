//! Global Descriptor Table (GDT)

use core::arch::asm;
use core::mem;
use core::ptr::addr_of;

use super::PrivilegeLevel;

const GDT_ENTRIES: usize = 3;

pub(super) const GDT_KERNEL_CODE: u16 = 1;
const GDT_KERNEL_DATA: u16 = 2;

static GDT: [GdtEntry; GDT_ENTRIES] = [
    // Null Descriptor
    GdtEntry::new(0, GdtEntryFlags::empty()),
    // Kernel Code Segment
    GdtEntry::new(
        GdtAccessFlags::PRESENT
            | GdtAccessFlags::KERNEL
            | GdtAccessFlags::DESCRIPTOR_TYPE
            | GdtAccessFlags::EXECUTABLE
            | GdtAccessFlags::RW
            | GdtAccessFlags::ACCESSED,
        GdtEntryFlags::LONG_MODE,
    ),
    // Kernel Data Segment
    GdtEntry::new(
        GdtAccessFlags::PRESENT
            | GdtAccessFlags::KERNEL
            | GdtAccessFlags::DESCRIPTOR_TYPE
            | GdtAccessFlags::RW
            | GdtAccessFlags::ACCESSED,
        GdtEntryFlags::LONG_MODE,
    ),
];

bitflags::bitflags! {
    #[derive(Copy, Clone)]
    /// A GDT entry's flags.
    struct GdtEntryFlags: u8 {
        /// Indicates the entry is a 32-bit segment.
        const PROTECTED_MODE = 1 << 6;
        /// Indicates the entry is a 64-bit segment.
        const LONG_MODE = 1 << 5;
    }
}

/// The flags present in the GDT entry's access byte.
#[derive(Copy, Clone)]
struct GdtAccessFlags;

impl GdtAccessFlags {
    /// This flag is set automatically by the CPU if the segment is accessed.
    const ACCESSED: u8 = 1 << 0;
    /// Direction/Conforming flag:
    /// - If the entry is a data segment, this flag indicates that the segment grows downward.
    /// - If the entry is a code segment, this flag indiicates if code can be executed from a lower
    ///   privilege level.
    const DC: u8 = 1 << 2;
    /// Descriptor type: If set this descriptor is a code or data segment.
    const DESCRIPTOR_TYPE: u8 = 1 << 4;
    /// Executable bit:  If set this segment is a code segment
    const EXECUTABLE: u8 = 1 << 3;
    /// Kernel privilege flag: This indicates the segment is a kernel segment.
    const KERNEL: u8 = 0 << 5;
    /// Present flag: Must be set for any valid segment
    const PRESENT: u8 = 1 << 7;
    /// Read/wWite flag:
    /// - If the entry is a data segment, this flag indicates whether or not the segment can be
    ///   written to.
    /// - If the entry is a code segment, this flag indicates whether or not the segment can be read
    ///   from.
    const RW: u8 = 1 << 1;
    /// User privile flag: This indicates the segment is a user segment.
    const USER: u8 = 3 << 5;
}

/// 8-byte entry in the GDT.
#[derive(Copy, Clone)]
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
    /// Construct a new GDT entry with the given access byte and flags.
    const fn new(access: u8, flags: GdtEntryFlags) -> Self {
        // Limit and base can be set to 0 as they're ignored on x86_64. We only care about the flags
        // and access byte.
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

/// The GDT descriptor contains the size and location of the GDT in memory. This structure is loaded
/// into the CPU via the `lgdt` instruction.
#[repr(C, packed)]
struct GdtDescriptor {
    size: u16,
    offset: u64,
}

impl GdtDescriptor {
    /// Create a new GDT descriptor with the given size and offset.
    const fn new(size: u16, offset: u64) -> Self {
        Self { size, offset }
    }
}

/// A segment selector is an index into the GDT, containing the index of the GDT entry and the
/// desired privilege level.
#[repr(transparent)]
pub(super) struct SegmentSelector(u16);

impl SegmentSelector {
    /// Create a new segment selector with the given GDT index and privilege level.
    pub const fn new(index: u16, privilege: PrivilegeLevel) -> Self {
        Self(index << 3 | (privilege as u16))
    }
}

pub fn init() {
    log::debug!("GDT: Loading GDT");
    let gdt_descriptor = GdtDescriptor::new(
        (mem::size_of::<[GdtEntry; GDT_ENTRIES]>() - 1) as u16,
        addr_of!(GDT) as u64,
    );

    // Safety: The GDT is valid.
    unsafe {
        load_gdt(&gdt_descriptor);

        set_cs(SegmentSelector::new(
            GDT_KERNEL_CODE,
            PrivilegeLevel::Kernel,
        ));
        set_ds(SegmentSelector::new(
            GDT_KERNEL_DATA,
            PrivilegeLevel::Kernel,
        ));
        set_es(SegmentSelector::new(
            GDT_KERNEL_DATA,
            PrivilegeLevel::Kernel,
        ));
        set_fs(SegmentSelector::new(
            GDT_KERNEL_DATA,
            PrivilegeLevel::Kernel,
        ));
        set_gs(SegmentSelector::new(
            GDT_KERNEL_DATA,
            PrivilegeLevel::Kernel,
        ));
        set_ss(SegmentSelector::new(
            GDT_KERNEL_DATA,
            PrivilegeLevel::Kernel,
        ));
    }

    log::debug!("GDT: Loaded GDT at address {:x?}", addr_of!(GDT));
}

/// Load the GDT described by `descriptor` into the CPU
///
/// # Safety: `descriptor` must point to a valid GDT.
unsafe fn load_gdt(descriptor: &GdtDescriptor) {
    asm!("lgdt[{}]", in(reg) descriptor, options(nostack));
}

/// Set the CS register to the given segment selector via a far return
///
/// # Safety
/// `selector` must point to a GDT entry with the correct privilege level.
unsafe fn set_cs(selector: SegmentSelector) {
    asm!(
        "push {selector}",
        "lea {tmp}, [rip + 2f]",
        "push {tmp}",
        "retfq",
        "2:",
        selector = in(reg) u64::from(selector.0),
        tmp = lateout(reg) _,
    )
}

/// Set the DS register to the given segment selector
///
/// # Safety
/// `selector` must point to a GDT entry with the correct privilege level
unsafe fn set_ds(selector: SegmentSelector) {
    asm!("mov ds, {0:x}", in(reg) selector.0)
}

/// Set the ES register to the given segment selector
///
/// # Safety
/// `selector` must point to a GDT entry with the correct privilege level
unsafe fn set_es(selector: SegmentSelector) {
    asm!("mov es, {0:x}", in(reg) selector.0)
}

/// Set the FS register to the given segment selector
///
/// # Safety
/// `selector` must point to a GDT entry with the correct privilege level
unsafe fn set_fs(selector: SegmentSelector) {
    asm!("mov fs, {0:x}", in(reg) selector.0)
}

/// Set the GS register to the given segment selector
///
/// # Safety
/// `selector` must point to a GDT entry with the correct privilege level
unsafe fn set_gs(selector: SegmentSelector) {
    asm!("mov gs, {0:x}", in(reg) selector.0)
}

/// Set the SS register to the given segment selector
///
/// # Safety
/// `selector` must point to a GDT entry with the correct privilege level
unsafe fn set_ss(selector: SegmentSelector) {
    asm!("mov ss, {0:x}", in(reg) selector.0)
}
