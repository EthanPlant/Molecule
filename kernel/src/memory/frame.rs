//! Abstractions around physical memory frames

use core::fmt;
use core::marker::PhantomData;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use super::addr::{AddrError, PhysAddr};
use super::page::{PageSize, Size4K};

/// A Physical memory frame
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct PhysFrame<S: PageSize = Size4K> {
    start_addr: PhysAddr,
    size: PhantomData<S>,
}

impl<S: PageSize> PhysFrame<S> {
    /// Returns the frame that starts at the given physical address.
    ///
    /// # Errors
    ///
    /// Returns [AddrError::NotAligned] if the address is not correctly aligned
    pub fn from_start(start_addr: PhysAddr) -> Result<Self, AddrError> {
        if !start_addr.is_aligned(S::SIZE) {
            return Err(AddrError::NotAligned);
        }

        // Safety: We've guaranteed the start address is aligned
        Ok(unsafe { Self::from_start_unchecked(start_addr) })
    }

    /// Returns the frame that starts at the given physical address.
    ///
    /// # Safety
    ///
    /// `start_addr` must be aligned to a frame start
    pub const unsafe fn from_start_unchecked(start_addr: PhysAddr) -> Self {
        Self {
            start_addr,
            size: PhantomData,
        }
    }

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

    /// Returns the size of the frame
    pub const fn size(&self) -> usize {
        S::SIZE
    }

    /// Returns a range of frames.
    pub fn range(start: Self, end: Self) -> PhysFrameRange<S> {
        PhysFrameRange { start, end }
    }

    /// /// Returns a range of frames, inclusive.
    pub fn range_inclusive(start: Self, end: Self) -> PhysFrameRangeInclusive<S> {
        PhysFrameRangeInclusive { start, end }
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

impl<S: PageSize> PhysFrameRange<S> {
    /// Returns whether the range contains no frames
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Returns the number of frames in the range.
    pub fn len(&self) -> usize {
        if !self.is_empty() {
            self.end - self.start
        } else {
            0
        }
    }

    /// Returns the size of the range in bytes.
    pub fn size(&self) -> usize {
        S::SIZE * self.len()
    }
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

impl<S: PageSize> PhysFrameRangeInclusive<S> {
    /// Returns whether the range contains no frames
    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }

    /// Returns the number of frames in the range.
    pub fn len(&self) -> usize {
        if !self.is_empty() {
            self.end - self.start + 1
        } else {
            0
        }
    }

    /// Returns the size of the range in bytes.
    pub fn size(&self) -> usize {
        S::SIZE * self.len()
    }
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
