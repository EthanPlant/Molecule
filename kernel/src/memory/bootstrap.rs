use core::{
    alloc::{AllocError, Allocator, Layout},
    ptr::NonNull,
};

use limine::{memory_map, response::MemoryMapResponse};
use spin::mutex::Mutex;

use crate::memory::addr::PhysAddr;

use super::addr::align_up;

pub struct BootstrapAlloc {
    memory_ranges: Mutex<&'static mut [&'static mut memory_map::Entry]>,
}

impl BootstrapAlloc {
    pub fn new(memory_ranges: &'static mut MemoryMapResponse) -> Self {
        Self {
            memory_ranges: Mutex::new(memory_ranges.entries_mut()),
        }
    }

    fn allocate(&self, size: usize) -> *mut u8 {
        let size = align_up(size, 4096);
        for range in self.memory_ranges.lock().iter_mut() {
            if range.length as usize >= size {
                let addr = range.base as usize;
                range.base += size as u64;
                range.length -= size as u64;

                log::trace!("Allocated {} bytes at {:x?}", size, addr);

                return PhysAddr::new(addr).as_hddm_virt().as_mut_ptr();
            }
        }

        unreachable!("Bootstrap allocator out of memory");
    }
}

#[derive(Clone, Copy)]
pub struct BootstrapAllocRef<'a> {
    inner: &'a BootstrapAlloc,
}

impl<'a> BootstrapAllocRef<'a> {
    pub fn new(inner: &'a BootstrapAlloc) -> Self {
        Self { inner }
    }
}

unsafe impl Allocator for BootstrapAllocRef<'_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let inner = self.inner;

        let aligned_size = align_up(layout.size() as _, layout.align() as _);
        let ptr = inner.allocate(aligned_size);

        let ptr = unsafe { NonNull::new_unchecked(ptr) };
        Ok(NonNull::slice_from_raw_parts(ptr, aligned_size))
    }

    unsafe fn deallocate(&self, _ptr: NonNull<u8>, _layout: Layout) {
        unreachable!("Bootstrap allocator can not deallocate");
    }
}
