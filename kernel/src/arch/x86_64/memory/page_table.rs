//! Abstractions for page tables and page table entries.

use alloc::fmt;
use core::ops::{Index, IndexMut};

use crate::memory::addr::PhysAddr;
use crate::memory::frame::PhysFrame;
use crate::memory::page::{PageSize, Size4K};

const ENTRY_COUNT: usize = 512;

bitflags::bitflags! {
    /// Possible flags for a page table entry.
    #[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
    pub struct PageTableFlags: u64 {
        /// The mapped frame or page table is loaded in memory.
        const PRESENT = 1;
        /// Controls whether writes to the mapped frames are allowed.
        ///
        /// If this bit is unset in a level 1 page table entry, the mapped frame is read-only.
        /// If this bit is unset in a higher level page table entry, the complete range of mapped pages is read-only.
        const WRITABLE = 1 << 1;
        /// Controls whether accesses from userspace are permitted.
        const USER_ACCESSIBLE = 1 << 2;
        /// If this bit is set, a "write-through" policy is used for the cache, else a "write-back" policy is used.
        const WRITE_THROUGH = 1 << 3;
        /// Disables caching for the entry.
        const NO_CACHE = 1 << 4;
        /// Set by the CPU when the mapped frame or page table is accessed.
        const ACCESSED = 1 << 5;
        /// Set by the CPU on a write to the mapped frame.
        const DIRTY = 1 << 6;
        /// Specifies that the entry maps a huge frame instead of a page table. Only allowed in P2 (for 2 MiB pages) or P3 (for 1 GiB pages) tables.
        const HUGE_PAGE = 1 << 7;
        /// Indicates the mapping is prsent in all address spaces, so it isn't flushed from the TLB on an address space switch.
        const GLOBAL = 1 << 8;
        /// Forbid code execution from the mapped frames.
        const NO_EXECUTE = 1 << 63;
    }
}

/// Error returned by [PageTableEntry::frame]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrameError {
    /// The entry doesn't have the [PageTableFlags::PRESENT] flag set, so it isn't currently mapped
    /// to a frame.
    FrameNotPresent,
    /// The entry has the [PageTableFlags::HUGE_PAGE] flag set. [PageTableEntry::frame] always
    /// returns a standard 4 KiB frame so a huge frame can't be returned.
    HugeFrame,
}

/// A page table entry
#[derive(Clone, Copy, Default)]
#[repr(transparent)]
pub struct PageTableEntry {
    entry: u64,
}

impl PageTableEntry {
    /// Create a new unused page table entry
    pub const fn new() -> Self {
        PageTableEntry { entry: 0 }
    }

    /// Returns whether this entry is zero.
    pub const fn is_unused(&self) -> bool {
        self.entry == 0
    }

    /// Sets this entry to zero.
    pub fn set_unused(&mut self) {
        self.entry = 0;
    }

    /// Returns the flags of this entry.
    pub const fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.entry)
    }

    /// Returns the physical address mapped by this entry.
    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.entry as usize & 0x000f_ffff_ffff_f000)
    }

    /// Returns the physical frame mapped by this entry.
    ///
    /// # Errors
    ///
    /// - [FrameError::FrameNotPresent] if the entry doesn't have the [PageTableFlags::PRESENT] flag
    ///   set.
    /// - [FrameError::HugeFrame] if the entry has the [PageTableFlags::HUGE_PAGE] flag seet. For
    ///   huge pages use the [PageTableEntry::addr] function.
    pub fn frame(&self) -> Result<PhysFrame, FrameError> {
        if !self.flags().contains(PageTableFlags::PRESENT) {
            Err(FrameError::FrameNotPresent)
        } else if self.flags().contains(PageTableFlags::HUGE_PAGE) {
            Err(FrameError::HugeFrame)
        } else {
            Ok(PhysFrame::containing_addr(self.addr()))
        }
    }

    /// Map the entry to the specified physical address with the specified flags.
    ///
    /// # Panics
    ///
    /// This function panics if the address is not aligned to 4 KiB.
    pub fn set(&mut self, addr: PhysAddr, flags: PageTableFlags) {
        assert!(addr.is_aligned(Size4K::SIZE));
        self.entry = (addr.as_usize()) as u64 | flags.bits()
    }

    /// Maps the entry to the specified physical frame with the specified flags.
    ///
    /// # Panics
    ///
    /// This function panics if the [PageTableFlags::HUGE_PAGE] flag is set.
    pub fn set_frame(&mut self, frame: &PhysFrame, flags: PageTableFlags) {
        assert!(!flags.contains(PageTableFlags::HUGE_PAGE));
        self.set(frame.start_addr(), flags);
    }

    /// Sets the flags of the entry.
    pub fn set_flags(&mut self, flags: PageTableFlags) {
        self.entry = self.addr().as_usize() as u64 | flags.bits()
    }
}

impl fmt::Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PageTableEntry")
            .field("addr", &self.addr())
            .field("flags", &self.flags())
            .finish()
    }
}

/// Represents a page table.
///
/// This struct implements the [Index] and [IndexMut] traits so the entries can be accessed through
/// index operations. For example, `page_table[15]` returns the 16th page table entry.
///
/// Note that while this type implements [Clone], users must be careful not to introduce mutable
/// aliasing by using the cloned pages.
#[repr(align(4096))]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT],
}

impl PageTable {
    /// Creates an empty page table
    pub const fn new() -> Self {
        const EMPTY: PageTableEntry = PageTableEntry::new();
        Self {
            entries: [EMPTY; ENTRY_COUNT],
        }
    }

    /// Clears all entries
    pub fn zero(&mut self) {
        for entry in self.iter_mut() {
            entry.set_unused();
        }
    }

    /// Returns an iterator over the entries of the page table.
    pub fn iter(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }

    /// Returns an iterator that allows modifying the entries of the page table.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }

    /// Checks if the page table is empty
    pub fn is_empty(&self) -> bool {
        self.iter().all(|entry| entry.is_unused())
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

impl Index<PageTableIndex> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: PageTableIndex) -> &Self::Output {
        &self.entries[index.0 as usize]
    }
}

impl IndexMut<PageTableIndex> for PageTable {
    fn index_mut(&mut self, index: PageTableIndex) -> &mut Self::Output {
        &mut self.entries[index.0 as usize]
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::new()
    }
}

/// A 9-bit index into a page table. Guaranteed to only ever contain 0..512
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PageTableIndex(u16);

impl PageTableIndex {
    /// Creates a new index from the given value.
    ///
    /// # Panics
    ///
    /// This function panics if the given value is >= 512.
    pub const fn new(index: u16) -> Self {
        assert!((index as usize) < ENTRY_COUNT);
        Self(index)
    }

    /// Creates a new index from the given value. Throws away bits if the value is >= 512.
    pub const fn new_truncate(index: u16) -> Self {
        Self(index % ENTRY_COUNT as u16)
    }
}

/// A 12-bit offset into a 4 KiB page. Guaranteed to only ever contain 0..4096
pub struct PageOffset(u16);

impl PageOffset {
    /// Creates a new offset from the given value. Panics if the passed value >= 4096.
    pub fn new(offset: u16) -> Self {
        assert!(offset < (1 << 12));
        Self(offset)
    }

    /// Creates a new offset from the given value. Throws away bits if the value is >= 4096.
    pub fn new_truncate(offset: u16) -> Self {
        Self(offset % (1 << 12))
    }
}
