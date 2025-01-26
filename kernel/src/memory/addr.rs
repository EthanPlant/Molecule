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
    fn from(addr: VirtAddr) -> usize {
        addr.0
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
}

impl From<PhysAddr> for usize {
    fn from(addr: PhysAddr) -> usize {
        addr.0
    }
}

impl From<PhysAddr> for VirtAddr {
    fn from(addr: PhysAddr) -> VirtAddr {
        VirtAddr::new(addr.0 + HHDM_OFFSET.get().unwrap().0)
    }
}

impl From<VirtAddr> for PhysAddr {
    fn from(addr: VirtAddr) -> PhysAddr {
        PhysAddr::new(addr.0 - HHDM_OFFSET.get().unwrap().0)
    }
}
