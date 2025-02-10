//! Heap allocation

use crate::{arch::paging::page_table::PageTableFlags, GLOBAL_ALLOC};

use super::{
    addr::VirtAddr,
    frame::{FrameAllocator, FRAME_ALLOCATOR},
    page::Page,
    MapError, PageSize, PageSize4K, VirtualMemoryManager,
};

pub const HEAP_START: usize = 0xFFFF_FE80_0000_0000;
pub const HEAP_SIZE: usize = 8 * 1024 * 1024; // 8 MB should be enough for now

/// Initialize the heap
///
/// # Error
/// Returns [`MapError::AllocationFailed`] if we can not allocate the frames required for the heap.
pub fn init_heap(mapper: &mut impl VirtualMemoryManager) -> Result<(), MapError> {
    let heap_start_page = Page::containing_addr(VirtAddr::new(HEAP_START));
    let heap_end_page = Page::containing_addr(VirtAddr::new(HEAP_START + HEAP_SIZE));

    let mut page = heap_start_page;
    while page != heap_end_page {
        let frame = FRAME_ALLOCATOR
            .allocate_frame()
            .ok_or(MapError::AllocationFailed)?;
        mapper.map_page(
            page,
            frame,
            PageTableFlags::PRESENT | PageTableFlags::WRITEABLE,
        )?;
        page = Page::containing_addr(page.start_addr() + PageSize4K::SIZE);
    }

    // Safety: This function is only called once, and the heap memory is reserved and will not be used by anything else.
    unsafe {
        GLOBAL_ALLOC.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    Ok(())
}
