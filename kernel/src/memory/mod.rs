use addr::{PhysAddr, VirtAddr};
use frame::{Frame, FrameError};
use page::Page;

use crate::arch::paging::page_table::PageTableFlags;

pub mod addr;
pub mod alloc;
pub mod bootstrap;
pub mod frame;
pub mod memmap;
pub mod page;

pub use frame::total_memory;

pub trait PageSize: Copy + Eq + PartialOrd + Ord {
    const SIZE: usize;
    const SIZE_STR: &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PageSize4K {}

impl PageSize for PageSize4K {
    const SIZE: usize = 4096;
    const SIZE_STR: &'static str = "4KiB";
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum MapError {
    /// Failed to allocate space for a page-frame mapping
    AllocationFailed,
    /// Attempted to map an already mapped frame.
    PageAlreadyMapped(Frame),
}

#[derive(Debug)]
pub enum UnmapError {
    ParentEntryHugePage,
    /// Attempting to unmap an already unmapped page.
    PageNotMapped,
}

/// A trait for common virtual memory operations.
#[allow(dead_code)]
pub trait VirtualMemoryManager {
    /// Translate a single virtual address to a physical address. Returns `None` if the virtual address is unmapped.
    fn translate_addr(&self, addr: VirtAddr) -> Option<PhysAddr>;
    /// Translate a page to a physical frame.
    ///
    /// # Errors
    /// - Returns [`FrameError::FrameNotPresent`] if the page is unmapped.
    /// - Returns [`FrameError::HugePageNotSupported`] if the page is a large page.
    fn translate_page(&self, page: Page) -> Result<Frame, FrameError>;

    /// Map a page of virtual memory to a physical frame, returning the newly mapped frame.
    ///
    /// # Errors
    /// Returns [`MapError`] if an error is encounted during mapping.
    fn map_page(&self, page: Page, frame: Frame, flags: PageTableFlags) -> Result<Frame, MapError>;
    /// Unmap a page of virtual memory, returning the frame that was just unmapped.
    ///
    /// # Errors
    /// Returns [`UnmapError`] if an error is encounted while unmapping.
    fn unmap_page(&self, page: Page) -> Result<Frame, UnmapError>;
}
