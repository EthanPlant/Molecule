use limine::memory_map;

use super::addr::PhysAddr;

#[derive(Debug, PartialEq, Eq)]
pub enum MemoryRegionType {
    Free,
}

/// An in-memory representation of a region in memory.
#[derive(Debug)]
pub struct MemoryRegion {
    /// The starting address of the region.
    pub base: PhysAddr,
    /// The region's size in bytes.
    pub size: usize,
    pub region_type: MemoryRegionType,
}

pub struct MemoryRegionIter<'a> {
    pub iter: core::slice::Iter<'a, &'a memory_map::Entry>,
    pub cursor_base: PhysAddr,
    pub cursor_end: PhysAddr,
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
                self.cursor_base = PhysAddr::new(entry.base as usize).align_up(4096_usize);
                self.cursor_end = PhysAddr::new(entry.base as usize + entry.length as usize);
            } else {
                return None;
            }
        }

        let region_type = MemoryRegionType::Free;

        let region = MemoryRegion {
            base: self.cursor_base,
            size: self.cursor_end - self.cursor_base,
            region_type,
        };
        self.cursor_base = self.cursor_end.align_up(4096_usize);
        Some(region)
    }
}
