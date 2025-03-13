//! x86_64 specific virtual memory management

use address_space::AddressSpace;
use page_table::{PageOffset, PageTable, PageTableIndex};

use crate::memory::addr::VirtAddr;

pub(super) mod address_space;
pub(super) mod heap;
mod page_table;

/// Returns the currently used level 4 page table.
fn active_level_4_table() -> &'static PageTable {
    let address_space = AddressSpace::this();
    address_space.page_table()
}

impl VirtAddr {
    /// Get the level 4 page table index from this address.
    pub fn p4_index(&self) -> PageTableIndex {
        PageTableIndex::new_truncate((self.as_usize() >> 12 >> 9 >> 9 >> 9) as u16)
    }

    /// Get the level 3 page table index from this address.
    pub fn p3_index(&self) -> PageTableIndex {
        PageTableIndex::new_truncate((self.as_usize() >> 12 >> 9 >> 9) as u16)
    }

    /// Get the level 2 page table index from this address.
    pub fn p2_index(&self) -> PageTableIndex {
        PageTableIndex::new_truncate((self.as_usize() >> 12 >> 9) as u16)
    }

    /// Get the level 1 page table index from this address.
    pub fn p1_index(&self) -> PageTableIndex {
        PageTableIndex::new_truncate((self.as_usize() >> 12) as u16)
    }

    /// Get the level page offset from this address.
    pub fn page_offset(&self) -> PageOffset {
        PageOffset::new_truncate((self.as_usize()) as u16)
    }
}
