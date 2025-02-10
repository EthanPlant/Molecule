use core::ops::{Add, AddAssign, Sub};

use spin::Once;

pub static HHDM_OFFSET: Once<VirtAddr> = Once::new();

#[derive(Debug, Copy, Clone)]
pub enum AddrError {
    NullPointer,
    NotAligned,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(usize);

impl VirtAddr {
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    pub const fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    pub fn read<'a, T: Sized>(&self) -> Result<&'a T, AddrError> {
        self.valid_read::<T>()?;

        Ok(unsafe { *self.as_ptr() })
    }

    pub fn as_bytes(&self, bytes: usize) -> Result<&[u8], AddrError> {
        self.valid_read::<&[u8]>()?;
        Ok(unsafe { core::slice::from_raw_parts(self.as_ptr(), bytes) })
    }

    pub fn read_mut<T: Sized>(&self) -> Result<&mut T, AddrError> {
        self.valid_read::<T>()?;

        Ok(unsafe { &mut *self.as_mut_ptr() })
    }

    pub fn as_bytes_mut(&self, bytes: usize) -> Result<&mut [u8], AddrError> {
        self.valid_read::<&[u8]>()?;
        Ok(unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), bytes) })
    }

    pub fn align_down<T: Into<usize>>(self, align: T) -> Self {
        Self(align_down(self.0, align.into()))
    }

    fn valid_read<T: Sized>(&self) -> Result<(), AddrError> {
        let raw = self.as_ptr::<T>();

        if raw.is_null() {
            return Err(AddrError::NullPointer);
        } else if !raw.is_aligned() {
            return Err(AddrError::NotAligned);
        }

        Ok(())
    }
}

impl From<VirtAddr> for usize {
    fn from(value: VirtAddr) -> Self {
        value.0
    }
}

impl Add<usize> for VirtAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(usize);

impl PhysAddr {
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    pub fn as_hddm_virt(self) -> VirtAddr {
        VirtAddr::new(self.0 + HHDM_OFFSET.get().unwrap().0)
    }

    pub fn align_up<T: Into<usize>>(self, align: T) -> Self {
        Self(align_up(self.0, align.into()))
    }

    pub fn align_down<T: Into<usize>>(self, align: T) -> Self {
        PhysAddr(align_down(self.0, align.into()))
    }

    pub fn is_aligned(self, align: usize) -> bool {
        self == self.align_up(align)
    }
}

impl Add<usize> for PhysAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<usize> for PhysAddr {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl Sub for PhysAddr {
    type Output = usize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0 - rhs.0
    }
}

impl From<PhysAddr> for usize {
    fn from(addr: PhysAddr) -> usize {
        addr.0
    }
}

impl From<VirtAddr> for PhysAddr {
    fn from(addr: VirtAddr) -> PhysAddr {
        PhysAddr::new(addr.0 - HHDM_OFFSET.get().unwrap().0)
    }
}

pub fn align_up(addr: usize, align: usize) -> usize {
    let mask = align - 1;
    if addr & mask == 0 {
        addr
    } else {
        (addr | mask) + 1
    }
}

pub fn align_down(addr: usize, align: usize) -> usize {
    addr & !(align - 1)
}
