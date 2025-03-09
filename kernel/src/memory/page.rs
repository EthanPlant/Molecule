//! Abstractions around virtual memory pages.

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
