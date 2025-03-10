//! Virtual address space management

use alloc::alloc::AllocError;
use core::arch::asm;
use core::mem::{self, MaybeUninit};

use super::active_level_4_table;
use super::page_table::{self, FrameError, PageTable, PageTableFlags};
use crate::memory::addr::PhysAddr;
use crate::memory::frame::PhysFrame;
use crate::memory::frame_allocator::{self, get_frame_allocator, FrameAllocator};
use crate::memory::page::{Page, Size4K};

/// Error returned if page mapping failed.
#[derive(Debug)]
pub enum MapError {
    /// Failed to allocate space for mapping.
    AllocationFailed,
    /// Attempted to map a page that's already mapped to a frame.
    PageAlreadyMapped(PhysFrame),
}

/// Error returned if unmapping a page failed.
#[derive(Debug)]
pub enum UnmapError {
    /// Attempted to unmap a huge page.
    HugePage,
    /// Attempting to unmap a page that isn't already mapped.
    PageNotMapped,
}

/// A virtual address space.
#[derive(Debug)]
pub struct AddressSpace {
    cr3: PhysAddr,
}

impl AddressSpace {
    /// Create a new virtual address space. In all virtual addres spaces, the higher-half kernel
    /// mappings will be present, while the lower-half mappings will be available for userspace
    /// applications.
    ///
    /// # Error
    ///
    /// Returns [AllocError] if we failed to allocate space for the new address space's page tables.
    pub fn new() -> Result<Self, AllocError> {
        let frame: PhysFrame<Size4K> = get_frame_allocator().allocate_frame().ok_or(AllocError)?;
        let mut page_table = PageTable::default();
        let current_table = active_level_4_table();

        // Copy the higher-half kernel pages.
        for i in 256..512 {
            page_table[i] = current_table[i];
        }

        // Safety: We can safely drop our new page table into the newly allocated frame.
        unsafe {
            let ptr: *mut MaybeUninit<PageTable> = frame.start_addr().as_hhdm_virt().as_mut_ptr();
            *(ptr.cast::<PageTable>()) = MaybeUninit::new(page_table).assume_init();
        }

        log::debug!(
            "VMM: Allocated new virtual memory space at {:x}",
            frame.start_addr()
        );

        Ok(Self {
            cr3: frame.start_addr(),
        })
    }

    /// Returns the currently active address space.
    pub fn this() -> Self {
        let cr3 = {
            let value: u64;
            // Safety: Reading from CR3 is always valid and will always return a pointer to the
            // current page table.
            unsafe { asm!("mov {}, cr3", out(reg) value, options(nomem)) }

            PhysAddr::new(value as usize & 0x000f_ffff_ffff_f000)
        };

        Self { cr3 }
    }

    /// Switch to this address space.
    pub fn switch(&self) {
        log::debug!("VMM: Switching to memory space at {:x}", self.cr3);
        let cr3 = self.cr3.as_usize() as u64;

        // Safety: `cr3` always points to a valid level 4 page table, and can be safely loaded into
        // CR3
        unsafe { asm!("mov cr3, {}", in(reg) cr3, options(nostack)) }
    }

    /// Returns a reference to the page table for this address space.
    pub fn page_table(&self) -> &'static PageTable {
        // Safety: `cr3` always points to a page table.
        unsafe { &*(self.cr3.as_hhdm_virt().as_ptr()) }
    }

    /// Returns a mutable reference to the page table for this address space.
    pub fn page_table_mut(&mut self) -> &'static mut PageTable {
        // Safety: `cr3` always points to a page table.
        unsafe { &mut *(self.cr3.as_hhdm_virt().as_mut_ptr()) }
    }

    /// Map a page of virtual memory to a physical frame, returning the newly mapped frame.
    ///
    /// # Errors
    /// - [MapError::AllocationFailed] if we fail to allocate space for the page table entries.
    /// - [MapError::PageAlreadyMapped] if we attempt to map an already mapped page.
    pub fn map_page(&mut self, page: Page, frame: PhysFrame) -> Result<PhysFrame, MapError> {
        let indicies = [
            page.start_addr().p4_index(),
            page.start_addr().p3_index(),
            page.start_addr().p2_index(),
        ];

        let mut page_table = self.page_table_mut();
        // Attempt to allocate
        for index in indicies {
            let entry = &mut page_table[index];
            if entry.is_unused() {
                if let Some(table_frame) = get_frame_allocator().allocate_frame() {
                    entry.set_frame(
                        &table_frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::USER_ACCESSIBLE,
                    );
                    // Safety: We've just allocated this frame and can use it to initialize a new
                    // page table
                    unsafe {
                        let new_table: *mut MaybeUninit<PageTable> =
                            entry.addr().as_hhdm_virt().as_mut_ptr();
                        *(new_table.cast::<PageTable>()) =
                            MaybeUninit::new(PageTable::default()).assume_init();
                    }
                } else {
                    return Err(MapError::AllocationFailed);
                }
            }

            // Safety: `entry.addr()` is guaranteed to point to an initialized page table.
            page_table = unsafe { &mut *(entry.addr().as_hhdm_virt().as_mut_ptr()) };
        }

        let entry = &mut page_table[page.start_addr().p1_index()];

        if !entry.is_unused() {
            return Err(MapError::PageAlreadyMapped(entry.frame().unwrap()));
        }

        entry.set_frame(
            &frame,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE,
        );

        Self::invalidate_page(page);

        log::debug!("Mapped {:?} to {:?}", page, entry.frame().unwrap());

        Ok(entry.frame().unwrap())
    }

    pub fn unmap_page(&mut self, page: Page) -> Result<(), UnmapError> {
        let indicies = [
            page.start_addr().p4_index(),
            page.start_addr().p3_index(),
            page.start_addr().p2_index(),
        ];

        let mut page_table = self.page_table_mut();
        for index in indicies {
            let entry = page_table[index];
            if entry.is_unused() {
                return Err(UnmapError::PageNotMapped);
            }

            // Safety: `entry.addr()` is guaranteed to point to an initialized page table.
            page_table = unsafe { &mut *(entry.addr().as_hhdm_virt().as_mut_ptr()) };
        }

        let entry = &mut page_table[page.start_addr().p1_index()];
        let frame = entry.frame().map_err(|err| match err {
            FrameError::FrameNotPresent => UnmapError::PageNotMapped,
            FrameError::HugeFrame => UnmapError::HugePage,
        })?;

        log::debug!("Unmapped {:?} from {:?}", page, frame);

        entry.set_unused();

        Self::invalidate_page(page);

        Ok(())
    }

    /// Invalidate a page's entry in the TLB.
    fn invalidate_page(page: Page) {
        // Safety: `invlpg` is always safe to call. Even if we `invlpg` on a still mapped page, this
        // will only result in a cache miss and the page will be readded to the TLB.
        unsafe {
            asm!("invlpg [{}]", in(reg) page.start_addr().as_usize() as u64, options(nostack, preserves_flags));
        }
    }
}
