//! Abstractions around physical memory frames

use core::fmt;
use core::marker::PhantomData;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use super::addr::PhysAddr;
use super::page::{PageSize, Size4K};

/// A Physical memory frame
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct PhysFrame<S: PageSize = Size4K> {
    start_addr: PhysAddr,
    size: PhantomData<S>,
}

impl<S: PageSize> PhysFrame<S> {
    /// Returns the frame that contains the given physical address.
    pub fn containing_addr(addr: PhysAddr) -> Self {
        Self {
            start_addr: addr.align_down(S::SIZE),
            size: PhantomData,
        }
    }

    /// Returns the start address of the frame
    pub const fn start_addr(&self) -> PhysAddr {
        self.start_addr
    }
}

impl<S: PageSize> fmt::Debug for PhysFrame<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "PhysFrame[{}]({:#x})",
            S::SIZE_STR,
            self.start_addr().as_usize()
        ))
    }
}

impl<S: PageSize> Add<usize> for PhysFrame<S> {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        PhysFrame::containing_addr(self.start_addr() + rhs * S::SIZE)
    }
}

impl<S: PageSize> AddAssign<usize> for PhysFrame<S> {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs
    }
}

impl<S: PageSize> Sub<usize> for PhysFrame<S> {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        PhysFrame::containing_addr(self.start_addr() - rhs * S::SIZE)
    }
}

impl<S: PageSize> SubAssign<usize> for PhysFrame<S> {
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs
    }
}

impl<S: PageSize> Sub<Self> for PhysFrame<S> {
    type Output = usize;

    fn sub(self, rhs: Self) -> Self::Output {
        (self.start_addr() - rhs.start_addr()) / S::SIZE
    }
}

/// A range of physical memory frames, exclusive of the upper bound
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhysFrameRange<S: PageSize = Size4K> {
    /// The start of the range, inclusive
    pub start: PhysFrame<S>,
    /// The end of the range, exclusive
    pub end: PhysFrame<S>,
}

impl<S: PageSize> Iterator for PhysFrameRange<S> {
    type Item = PhysFrame<S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let frame = self.start;
            self.start += 1;
            Some(frame)
        } else {
            None
        }
    }
}

/// A range of physical memory frames, inclusive of the upper bound
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhysFrameRangeInclusive<S: PageSize = Size4K> {
    /// The start of the range, inclusive
    pub start: PhysFrame<S>,
    /// The end of the range, exclusive
    pub end: PhysFrame<S>,
}

impl<S: PageSize> Iterator for PhysFrameRangeInclusive<S> {
    type Item = PhysFrame<S>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start <= self.end {
            let frame = self.start;
            self.start += 1;
            Some(frame)
        } else {
            None
        }
    }
}
