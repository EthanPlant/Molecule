//! A bootstrap allocator used to initialize the memory management system until the heap can be initialized.
//! This is a simple first-free-fit allocator with no ability to deallocate memory.

use core::{
    alloc::{AllocError, Allocator, Layout},
    ptr::NonNull,
};

use spin::mutex::Mutex;

use crate::memory::{PageSize, PageSize4K};

use super::{addr::align_up, memmap::MemoryRegion};

pub struct BootstrapAlloc {
    /// Free areas of memory that the allocator is allowed to use.
    pub memory_ranges: Mutex<&'static mut [MemoryRegion]>,
}

impl BootstrapAlloc {
    /// Create a new instance of the boot allocator.
    pub fn new(memory_ranges: &'static mut [MemoryRegion]) -> Self {
        Self {
            memory_ranges: Mutex::new(memory_ranges),
        }
    }

    /// Allocate `size` bytes, aligned up to 4 KiB.
    fn allocate(&self, size: usize) -> *mut u8 {
        let size = align_up(size, PageSize4K::SIZE);
        for range in self.memory_ranges.lock().iter_mut() {
            if range.size >= size {
                let addr = range.base;
                range.base += size;
                range.size -= size;

                return addr.as_hddm_virt().as_mut_ptr();
            }
        }

        unreachable!("Bootstrap allocator out of memory");
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BootstrapAllocRef {
    pub inner: *const BootstrapAlloc,
}

impl BootstrapAllocRef {
    pub fn new(inner: &BootstrapAlloc) -> Self {
        Self { inner }
    }

    pub fn get_inner(&self) -> &BootstrapAlloc {
        unsafe { &*self.inner }
    }
}

unsafe impl Allocator for BootstrapAllocRef {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let inner = self.get_inner();

        let aligned_size = align_up(layout.size() as _, layout.align() as _);
        let ptr = inner.allocate(aligned_size);

        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        Ok(NonNull::slice_from_raw_parts(ptr, aligned_size))
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        unreachable!("Bootstrap allocator can not deallocate");
    }
}

unsafe impl Send for BootstrapAllocRef {}
