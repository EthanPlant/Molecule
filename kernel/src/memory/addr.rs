//! Physical and virtual address manipulation

use core::ops::{Add, AddAssign, Sub};

use spin::Once;

/// The higher-half memory offset returned from Limine.
pub static HHDM_OFFSET: Once<VirtAddr> = Once::new();

#[derive(Debug, Copy, Clone)]
pub enum AddrError {
    /// Attempting to read from a null pointer.
    NullPointer,
    /// Address is not properly aligned to the type attempting to be read.
    NotAligned,
}

/// A representation of a virtual address.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct VirtAddr(usize);

impl VirtAddr {
    /// Create a new virtual address.
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    /// Create a null virtual address that points to 0.
    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Convert the address to a raw pointer.
    pub const fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    /// Convert the address to a mutable raw pointer.
    pub const fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    /// Reads `sizeof(T)` bytes from the virtual address, and returns a reference to the value.
    ///
    /// # Errors
    /// - Returns [`AddrError::NullPointer`] if the address is null.
    /// - Returns [`AddrError::NotAligned`] if the address is not aligned for the type.
    pub fn read<'a, T: Sized>(&self) -> Result<&'a T, AddrError> {
        self.valid_read::<T>()?;

        Ok(unsafe { *self.as_ptr() })
    }

    /// Reads `sizeof(T)` bytes from the virtual address, and returns a mutable reference to the value.
    ///
    /// # Errors
    /// - Returns [`AddrError::NullPointer`] if the address is null.
    /// - Returns [`AddrError::NotAligned`] if the address is not aligned for the type.
    pub fn read_mut<T: Sized>(&self) -> Result<&mut T, AddrError> {
        self.valid_read::<T>()?;

        Ok(unsafe { &mut *self.as_mut_ptr() })
    }

    /// /// Reads `bytes` bytes from the virtual address.
    ///
    /// # Errors
    /// - Returns [`AddrError::NullPointer`] if the address is null.
    /// - Returns [`AddrError::NotAligned`] if the address is not aligned for the type.
    pub fn as_bytes(&self, bytes: usize) -> Result<&[u8], AddrError> {
        self.valid_read::<&[u8]>()?;
        Ok(unsafe { core::slice::from_raw_parts(self.as_ptr(), bytes) })
    }

    /// /// Reads `bytes` bytes from the virtual address, and returns a mutable slice to them.
    ///
    /// # Errors
    /// - Returns [`AddrError::NullPointer`] if the address is null.
    /// - Returns [`AddrError::NotAligned`] if the address is not aligned for the type.
    pub fn as_bytes_mut(&self, bytes: usize) -> Result<&mut [u8], AddrError> {
        self.valid_read::<&[u8]>()?;
        Ok(unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), bytes) })
    }

    /// Aligns the address downwards to the given alignment.
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

/// A representation of a virtual address.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct PhysAddr(usize);

impl PhysAddr {
    /// Create a new virtual address.
    pub fn new(addr: usize) -> Self {
        Self(addr)
    }

    /// Create a null virtual address that points to 0.
    pub const fn null() -> Self {
        Self(0)
    }

    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Convert the address to a higher-half direct map [`VirtAddr`].
    pub fn as_hddm_virt(self) -> VirtAddr {
        VirtAddr::new(self.0 + HHDM_OFFSET.get().unwrap().0)
    }

    /// Align the address up to the given alignment.
    pub fn align_up<T: Into<usize>>(self, align: T) -> Self {
        Self(align_up(self.0, align.into()))
    }

    /// Align the address up to the given alignment.
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

/// Align upwards.
///
/// Returns the smallest `x` with alignment `align` such that `x` >= `addr`.
pub fn align_up(addr: usize, align: usize) -> usize {
    let mask = align - 1;
    if addr & mask == 0 {
        addr
    } else {
        (addr | mask) + 1
    }
}

/// Align upwards.
///
/// Returns the largest `x` with alignment `align` such that `x` <= `addr`.
///
/// # Panics
/// This function panics if the alignment is not a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
    assert!(align.is_power_of_two());
    addr & !(align - 1)
}
