//! Virtual and memory address manipulation

use core::fmt;
use core::ops::{Add, AddAssign, Sub, SubAssign};

use spin::Once;

/// Higher-half memory offset returned from Limine.
pub static HHDM_OFFSET: Once<VirtAddr> = Once::new();

#[derive(Debug, Copy, Clone)]
pub enum AddrError {
    /// Attempted to read from a null pointer
    NullPointer,
    /// Attempted to read from a pointer that is not properly aligned
    NotAligned,
}

/// A phyiscal memory address
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct PhysAddr(usize);

impl PhysAddr {
    /// Creates a new physical address.
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    /// Creates a null physical address that points to 0.
    pub const fn null() -> Self {
        Self(0)
    }

    /// Align the address upwards.
    ///
    /// # Panics
    ///
    /// This function panics if `align` is not a power of two.
    pub fn align_up(self, align: usize) -> Self {
        Self(align_up(self.0, align))
    }

    /// Align the address downwards.
    ///
    /// # Panics
    ///
    /// This function panics if `align` is not a power of two.
    pub fn align_down(self, align: usize) -> Self {
        Self(align_down(self.0, align))
    }

    /// Converts the physical address into a higher-half virtual address
    ///
    /// # Panics
    ///
    /// This function panics if [HHDM_OFFSET] hasn't been initialized yet.
    pub fn as_hhdm_virt(&self) -> VirtAddr {
        *HHDM_OFFSET.get().unwrap() + self.0
    }

    /// Converts the physical address into a `usize`.
    pub fn as_usize(&self) -> usize {
        self.0
    }

    /// Convienence method for checking if an address is null.
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Check whether the address is aligned.
    pub fn is_aligned(self, align: usize) -> bool {
        self.align_down(align) == self
    }
}

impl fmt::Debug for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PhysAddr")
            .field(&format_args!("{:#x}", self.0))
            .finish()
    }
}

impl fmt::Binary for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Binary::fmt(&self.0, f)
    }
}

impl fmt::LowerHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::Octal for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Octal::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl fmt::Pointer for PhysAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(self.0 as *const ()), f)
    }
}

impl Add<usize> for PhysAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl AddAssign<usize> for PhysAddr {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl Sub<usize> for PhysAddr {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self::new(self.0.checked_sub(rhs).unwrap())
    }
}

impl SubAssign<usize> for PhysAddr {
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs
    }
}

impl Sub<PhysAddr> for PhysAddr {
    type Output = usize;

    fn sub(self, rhs: PhysAddr) -> Self::Output {
        self.as_usize() - rhs.as_usize()
    }
}

impl From<usize> for PhysAddr {
    fn from(value: usize) -> Self {
        Self::new(value)
    }
}

impl From<PhysAddr> for usize {
    fn from(value: PhysAddr) -> Self {
        value.as_usize()
    }
}

/// A virtual memory address
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct VirtAddr(usize);

impl VirtAddr {
    /// Creates a new virtual address.
    pub const fn new(addr: usize) -> Self {
        Self(addr)
    }

    /// Creates a null address that points to 0.
    pub const fn null() -> Self {
        Self(0)
    }

    /// Creates a new virtual address from the given pointer.
    pub fn from_ptr<T: ?Sized>(ptr: *const T) -> Self {
        Self::new(ptr.cast::<()>() as usize)
    }

    /// Align the address upwards.
    ///
    /// # Panics
    ///
    /// This function panics if `align` is not a power of two.
    pub fn align_up(self, align: usize) -> Self {
        Self(align_up(self.0, align))
    }

    /// Align the address downwards.
    ///
    /// # Panics
    ///
    /// This function panics if `align` is not a power of two.
    pub fn align_down(self, align: usize) -> Self {
        Self(align_down(self.0, align))
    }

    /// Converts the address to a `usize`
    pub const fn as_usize(&self) -> usize {
        self.0
    }

    /// Converts the address to a raw pointer of type `T`.
    pub const fn as_ptr<T>(&self) -> *const T {
        self.0 as *const T
    }

    /// Converts the address to a mutable raw pointer of type `T`.
    pub const fn as_mut_ptr<T>(&self) -> *mut T {
        self.0 as *mut T
    }

    /// Convienence method for checking if an address is null.
    pub const fn is_null(&self) -> bool {
        self.0 == 0
    }

    /// Check whether the address is aligned.
    pub fn is_aligned(self, align: usize) -> bool {
        self.align_down(align) == self
    }

    /// Reads `sizeof(T)` bytes from the virtual address and returns a reference to the value.
    ///
    /// # Errors
    ///
    /// - Returns [AddrError::NullPointer] if the address is null.
    /// - Returns [AddrError::NotAligned] if the address is misaligned for `T`.
    pub fn read<'a, T: Sized>(&self) -> Result<&'a T, AddrError> {
        self.valid_read::<T>()?;

        // Safety: We've guaranteed the pointer is safe to dereference
        Ok(unsafe { *self.as_ptr() })
    }

    /// Reads `sizeof(T)` bytes from the virtual address and returns a mutable reference to the
    /// value.
    ///
    /// # Errors
    ///
    /// - Returns [AddrError::NullPointer] if the address is null.
    /// - Returns [AddrError::NotAligned] if the address is misaligned for `T`.
    pub fn read_mut<T: Sized>(&self) -> Result<&mut T, AddrError> {
        self.valid_read::<T>()?;

        // Safety: We've guaranteed the pointer is safe to dereference
        Ok(unsafe { &mut *self.as_mut_ptr() })
    }

    /// Reads `bytes` from the virtual address.
    ///
    /// # Errors
    ///
    /// - Returns [AddrError::NullPointer] if the address is null.
    /// - Returns [AddrError::NotAligned] if the address is misaligned for \[u8].
    pub fn as_bytes(&self, bytes: usize) -> Result<&[u8], AddrError> {
        self.valid_read::<&[u8]>()?;

        // Safety: We've guaranteed the pointer is safe to dereference
        Ok(unsafe { core::slice::from_raw_parts(self.as_ptr(), bytes) })
    }

    /// Reads `bytes` from the virtual address and returns a mutable slice to them.
    ///
    /// # Errors
    ///
    /// - Returns [AddrError::NullPointer] if the address is null.
    /// - Returns [AddrError::NotAligned] if the address is misaligned for \[u8].
    pub fn as_bytes_mut(&self, bytes: usize) -> Result<&mut [u8], AddrError> {
        self.valid_read::<&[u8]>()?;

        // Safety: We've guaranteed the pointer is safe to dereference
        Ok(unsafe { core::slice::from_raw_parts_mut(self.as_mut_ptr(), bytes) })
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

impl fmt::Debug for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("VirtAddr")
            .field(&format_args!("{:#x}", self.0))
            .finish()
    }
}

impl fmt::Binary for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Binary::fmt(&self.0, f)
    }
}

impl fmt::LowerHex for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::Octal for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Octal::fmt(&self.0, f)
    }
}

impl fmt::UpperHex for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::UpperHex::fmt(&self.0, f)
    }
}

impl fmt::Pointer for VirtAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&(self.0 as *const ()), f)
    }
}

impl Add<usize> for VirtAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.0 + rhs)
    }
}

impl AddAssign<usize> for VirtAddr {
    fn add_assign(&mut self, rhs: usize) {
        *self = *self + rhs;
    }
}

impl Sub<usize> for VirtAddr {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self::new(self.0.checked_sub(rhs).unwrap())
    }
}

impl SubAssign<usize> for VirtAddr {
    fn sub_assign(&mut self, rhs: usize) {
        *self = *self - rhs
    }
}

impl From<usize> for VirtAddr {
    fn from(value: usize) -> Self {
        Self::new(value)
    }
}

impl From<VirtAddr> for usize {
    fn from(value: VirtAddr) -> Self {
        value.as_usize()
    }
}

/// Align `addr` up to `align`.
///
/// # Panics
///
/// This function panics if `align` is not a power of two.
pub fn align_up(addr: usize, align: usize) -> usize {
    assert!(align.is_power_of_two());

    let mask = align - 1;
    if addr & mask == 0 {
        addr
    } else {
        (addr | mask) + 1
    }
}

/// Align `addr` down to `align`.
///
/// # Panics
///
/// This function panics if `align` is not a power of two.
pub fn align_down(addr: usize, align: usize) -> usize {
    assert!(align.is_power_of_two());
    addr & !(align - 1)
}
