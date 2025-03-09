//! Memory map management

use limine::memory_map;

use super::addr::PhysAddr;
use super::page::{PageSize, Size4K};

/// A type of memory region.
#[derive(Debug, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// This memory region is free and available for use.
    Free,
}

/// A region of memory.
#[derive(Debug)]
pub struct MemoryRegion {
    /// The start address of this region.
    pub base: PhysAddr,
    /// The size of the region in bytes.
    pub size: usize,
    /// The type of memory region.
    pub region_type: MemoryRegionType,
}

/// An iterator over regions in memory
pub struct MemoryRegionIter<'a> {
    iter: core::slice::Iter<'a, &'a memory_map::Entry>,
    cursor_base: PhysAddr,
    cursor_end: PhysAddr,
}

impl Iterator for MemoryRegionIter<'_> {
    type Item = MemoryRegion;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor_base >= self.cursor_end {
            if let Some(entry) = loop {
                let next = self.iter.next()?;

                if next.entry_type == memory_map::EntryType::USABLE {
                    break Some(next);
                }
            } {
                self.cursor_base = PhysAddr::new(entry.base as usize).align_up(Size4K::SIZE);
                self.cursor_end = PhysAddr::new(entry.base as usize + entry.length as usize);
            } else {
                return None;
            }
        }

        let region = MemoryRegion {
            base: self.cursor_base,
            size: self.cursor_end - self.cursor_base,
            region_type: MemoryRegionType::Free,
        };

        self.cursor_base = self.cursor_end.align_up(Size4K::SIZE);
        Some(region)
    }
}
