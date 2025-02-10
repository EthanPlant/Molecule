use addr::{PhysAddr, VirtAddr};
use frame::{Frame, FrameError};
use limine::memory_map::EntryType;
use page::Page;

pub mod addr;
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

pub trait VirtualMemoryManager {
    fn translate_addr(&self, addr: VirtAddr) -> Option<PhysAddr>;
    fn translate_page(&self, page: Page<PageSize4K>) -> Result<Frame, FrameError>;
}
