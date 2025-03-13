//! Abstractions around virtual memory pages.

use core::fmt;
use core::marker::PhantomData;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use super::addr::VirtAddr;

/// Trait for abstracting around page sizes
pub trait PageSize: Copy + Eq + PartialOrd + Ord {
    const SIZE: usize;
    const SIZE_STR: &'static str;
}

/// A 4 KiB page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size4K {}

impl PageSize for Size4K {
    const SIZE: usize = 4096;
    const SIZE_STR: &'static str = "4 KiB";
}

/// A 2 MiB page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size2M {}

impl PageSize for Size2M {
    const SIZE: usize = Size4K::SIZE * 512;
    const SIZE_STR: &'static str = "2 MiB";
}

/// A 1 GiB page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Size1G {}

impl PageSize for Size1G {
    const SIZE: usize = Size2M::SIZE * 512;
    const SIZE_STR: &'static str = "1 GiB";
}

/// A page of virtual memory.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct Page<S: PageSize = Size4K> {
    start_addr: VirtAddr,
    size: PhantomData<S>,
}

impl<S: PageSize> Page<S> {
    /// Returns the page that contains the given virtual address.
    pub fn containing_addr(addr: VirtAddr) -> Self {
        Page {
            start_addr: addr.align_down(S::SIZE),
            size: PhantomData,
        }
    }

    /// Returns the start address of the page.
    pub fn start_addr(&self) -> VirtAddr {
        self.start_addr
    }
}

impl<S: PageSize> fmt::Debug for Page<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "Page[{}]({:#x})",
            S::SIZE_STR,
            self.start_addr().as_usize()
        ))
    }
}

impl<S: PageSize> Add<usize> for Page<S> {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::containing_addr(self.start_addr() + rhs * S::SIZE)
    }
}

impl<S: PageSize> AddAssign<usize> for Page<S> {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs
    }
}

impl<S: PageSize> Sub<usize> for Page<S> {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self::containing_addr(self.start_addr() - rhs * S::SIZE)
    }
}

impl<S: PageSize> SubAssign<usize> for Page<S> {
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs
    }
}

impl<S: PageSize> Sub<Self> for Page<S> {
    type Output = usize;

    fn sub(self, rhs: Self) -> Self::Output {
        (self.start_addr() - rhs.start_addr()) / S::SIZE
    }
}

/// A range of pages with exclusive upper bound.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct PageRange<S: PageSize = Size4K> {
    pub start: Page<S>,
    pub end: Page<S>,
}

impl<S: PageSize> Iterator for PageRange<S> {
    type Item = Page<S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let page = self.start;
            self.start += 1;
            Some(page)
        } else {
            None
        }
    }
}

/// A range of pages with inclusive upper bound.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct PageRangeInclusive<S: PageSize = Size4K> {
    pub start: Page<S>,
    pub end: Page<S>,
}

impl<S: PageSize> Iterator for PageRangeInclusive<S> {
    type Item = Page<S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let page = self.start;
            let max_page_addr = VirtAddr::new(usize::MAX) - (S::SIZE - 1);
            if self.start.start_addr() < max_page_addr {
                self.start += 1;
            } else {
                self.start -= 1;
            }
            Some(page)
        } else {
            None
        }
    }
}
