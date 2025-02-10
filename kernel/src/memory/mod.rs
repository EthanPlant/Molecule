use addr::{PhysAddr, VirtAddr};
use frame::{Frame, FrameError};
use limine::memory_map::EntryType;
use page::Page;

use crate::arch::paging::page_table::PageTableFlags;

pub mod addr;
pub mod alloc;
pub mod bootstrap;
pub mod frame;
pub mod memmap;
pub mod page;

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
pub enum MapError {
    AllocationFailed,
    PageAlreadyMapped(Frame),
}

#[derive(Debug)]
pub enum UnmapError {
    ParentEntryHugePage,
    PageNotMapped,
    InvalidFrameAddress(Frame),
}

pub trait VirtualMemoryManager {
    fn translate_addr(&self, addr: VirtAddr) -> Option<PhysAddr>;
    fn translate_page(&self, page: Page) -> Result<Frame, FrameError>;

    fn map_page(&self, page: Page, frame: Frame, flags: PageTableFlags) -> Result<Frame, MapError>;
    fn unmap_page(&self, page: Page) -> Result<Frame, UnmapError>;
}
