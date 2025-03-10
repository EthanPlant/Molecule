//! A bootstrap allocator used to allocate memory for the memory management system's metadata.

use alloc::alloc::{AllocError, Allocator};
use core::alloc::Layout;
use core::ptr::NonNull;

use super::addr::align_up;
use super::mem_map::MemoryRegion;
use super::page::{PageSize, Size4K};
use crate::sync::Mutex;

/// A simple first-free-fit allocator with no ability to deallocate
pub(super) struct BootstrapAlloc {
    pub memory_regions: Mutex<&'static mut [MemoryRegion]>,
}

impl BootstrapAlloc {
    /// Create a new instance of the boot allocator with the given memory regions.
    pub fn new(memory_regions: &'static mut [MemoryRegion]) -> Self {
        Self {
            memory_regions: Mutex::new(memory_regions),
        }
    }

    /// Allocate `size` bytes and returns a pointer to the beginning of the allocation. Returns None
    /// if the allocator is out of memory.
    fn allocate(&self, size: usize) -> Option<*mut u8> {
        let size = align_up(size, Size4K::SIZE);
        for range in self.memory_regions.lock().iter_mut() {
            if range.size >= size {
                let addr = range.base;
                range.base += size;
                range.size -= size;

                log::debug!(
                    "Memory: Bootstrap allocator allocated {size} bytes at {:x}",
                    addr.as_hhdm_virt()
                );

                return Some(addr.as_hhdm_virt().as_mut_ptr());
            }
        }

        None
    }
}

/// A smart pointer around the bootstrap allocator
#[derive(Clone, Copy, Debug)]
pub(super) struct BootstrapAllocRef {
    inner: *const BootstrapAlloc,
}

impl BootstrapAllocRef {
    /// Create a new [BootstrapAllocRef].
    pub fn new(inner: &BootstrapAlloc) -> Self {
        Self { inner }
    }

    /// Get the inner [BootstrapAlloc].
    pub fn get_inner(&self) -> &BootstrapAlloc {
        unsafe { &*self.inner }
    }
}

// Safety: The bootstrap allocator is guaranteed to return memory that will be valid for the entire
// runtime of the kernel.
unsafe impl Allocator for BootstrapAllocRef {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let aligned_size = align_up(layout.size(), layout.align());
        let inner = self.get_inner();
        let ptr = inner.allocate(aligned_size);
        if ptr.is_none() {
            return Err(AllocError);
        }

        let ptr = ptr.unwrap();
        assert!(
            !ptr.is_null(),
            "Bootstrap allocator returned a null pointer"
        );
        // Safety: ptr is non-null.
        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        Ok(NonNull::slice_from_raw_parts(ptr, aligned_size))
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        unreachable!("Bootstrap allocator can not deallocate");
    }
}

// Safety: `BootstrapAlloc` safely wraps access to the memory regions behind a mutex.
unsafe impl Send for BootstrapAllocRef {}
