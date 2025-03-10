use super::address_space::{AddressSpace, MapError};
use super::page_table::PageTableFlags;
use crate::memory::addr::VirtAddr;
use crate::memory::frame_allocator::{get_frame_allocator, FrameAllocator};
use crate::memory::page::{Page, PageRange};
use crate::GLOBAL_ALLOC;

const HEAP_START: usize = 0xffff_fe80_0000_0000;
const HEAP_SIZE: usize = 1024 * 1024;

/// Initialize the heap.
///
/// # Error
/// Returns [MapError::AllocationFailed] if we can not allocate the frames required for the heap.
/// Returns [MapError::PageAlreadyMapped] if a page for the heap is already mapped.
pub fn init() -> Result<(), MapError> {
    let mut addr_space = AddressSpace::this();
    let heap_start = Page::containing_addr(VirtAddr::new(HEAP_START));
    let heap_end = Page::containing_addr(VirtAddr::new(HEAP_START + HEAP_SIZE));
    let range = PageRange {
        start: heap_start,
        end: heap_end,
    };
    for page in range {
        let frame = get_frame_allocator()
            .allocate_frame()
            .ok_or(MapError::AllocationFailed)?;
        addr_space.map_page(
            page,
            frame,
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
        )?;
    }

    // Safety: This function is only ever called once, `HEAP_START` is a valid pointer to the
    // beginning of the heap's memory, and this memory region will only be used by the heap.
    unsafe {
        GLOBAL_ALLOC.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
    }

    Ok(())
}
