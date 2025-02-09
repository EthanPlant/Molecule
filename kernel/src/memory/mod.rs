use addr::PhysAddr;
use limine::memory_map::EntryType;

pub mod addr;
pub mod bootstrap;
pub mod frame;
pub mod memmap;

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
