use core::arch::asm;

use page_table::{PageTable, PageTableFlags};

use crate::memory::{
    addr::{PhysAddr, VirtAddr, HHDM_OFFSET},
    frame::{Frame, FrameAllocator, FrameError, FRAME_ALLOCATOR},
    page::Page,
    MapError, PageSize4K, UnmapError, VirtualMemoryManager,
};

pub mod page_table;

impl VirtAddr {
    pub fn p4_index(self) -> u16 {
        (usize::from(self) >> 12 >> 9 >> 9 >> 9) as u16 % 512
    }

    pub fn p3_index(self) -> u16 {
        (usize::from(self) >> 12 >> 9 >> 9) as u16 % 512
    }

    pub fn p2_index(self) -> u16 {
        (usize::from(self) >> 12 >> 9) as u16 % 512
    }

    pub fn p1_index(self) -> u16 {
        (usize::from(self) >> 12) as u16 % 512
    }

    pub fn page_index(self) -> u16 {
        (usize::from(self)) as u16 % (1 << 12)
    }
}

pub fn init(mem_map: &mut limine::response::MemoryMapResponse) {
    FRAME_ALLOCATOR.init(mem_map);
}

#[derive(Debug)]
pub struct PageMap {
    pml4: PhysAddr,
}

impl PageMap {
    pub unsafe fn from_cr3(cr3: PhysAddr) -> Self {
        Self { pml4: cr3 }
    }

    fn page_table(&self) -> &mut PageTable {
        Self::page_table_from_addr(self.pml4)
    }

    fn page_table_from_addr(addr: PhysAddr) -> &'static mut PageTable {
        let virt = addr.as_hddm_virt();
        let table_ptr: *mut PageTable = virt.as_mut_ptr();
        unsafe { &mut *table_ptr }
    }

    fn invalidate_page(page: Page) {
        unsafe {
            asm!("invlpg [{}]", in(reg) usize::from(page.start_addr()), options(nostack, preserves_flags));
        }
    }
}

impl VirtualMemoryManager for PageMap {
    fn translate_addr(&self, addr: VirtAddr) -> Option<PhysAddr> {
        let indicies = [
            addr.p1_index(),
            addr.p2_index(),
            addr.p3_index(),
            addr.p4_index(),
        ];
        let mut frame = Frame::containing_addr(self.pml4);
        for (level, &index) in indicies.iter().enumerate().rev() {
            let virt = frame.start_addr().as_hddm_virt();
            let table_ptr: *const PageTable = virt.as_ptr();
            let page_table = unsafe { &*table_ptr };

            let entry = &page_table[index];
            frame = match entry.frame() {
                Ok(frame) => frame,
                Err(FrameError::FrameNotPresent) => return None,
                Err(FrameError::HugePageNotSupported) => {
                    assert!((level == 1), "Level {} has large page flag set", level + 1);
                    return Some(
                        entry.addr() + addr.p1_index() as usize + addr.page_index() as usize,
                    );
                }
                _ => unreachable!(),
            }
        }

        Some(frame.start_addr() + addr.page_index().into())
    }

    fn translate_page(&self, page: Page) -> Result<Frame, FrameError> {
        let indicies = [
            page.start_addr().p4_index(),
            page.start_addr().p3_index(),
            page.start_addr().p2_index(),
            page.start_addr().p1_index(),
        ];

        let mut frame = Frame::containing_addr(self.pml4);
        for index in indicies {
            let virt = frame.start_addr().as_hddm_virt();
            let table_ptr: *const PageTable = virt.as_ptr();
            let page_table = unsafe { &*table_ptr };

            let entry = &page_table[index];
            frame = entry.frame()?;
        }

        Ok(frame)
    }

    fn map_page(&self, page: Page, frame: Frame, flags: PageTableFlags) -> Result<Frame, MapError> {
        let indicies = [
            page.start_addr().p4_index(),
            page.start_addr().p3_index(),
            page.start_addr().p2_index(),
        ];

        let mut page_table = self.page_table();
        for index in indicies {
            let entry = &mut page_table[index];
            if entry.is_unused() {
                if let Some(table_frame) = FRAME_ALLOCATOR.allocate_frame() {
                    entry.set_frame(
                        table_frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITEABLE
                            | PageTableFlags::USER_ACCESSIBLE,
                    );
                    Self::page_table_from_addr(entry.addr()).zero();
                } else {
                    return Err(MapError::AllocationFailed);
                }
            }

            page_table = Self::page_table_from_addr(entry.addr());
        }

        let entry = &mut page_table[page.start_addr().p1_index()];

        if !entry.is_unused() {
            return Err(MapError::PageAlreadyMapped(entry.frame().unwrap()));
        }

        entry.set_frame(frame, flags);

        Self::invalidate_page(page);

        Ok(entry.frame().unwrap())
    }

    fn unmap_page(&self, page: Page) -> Result<Frame, UnmapError> {
        let indicies = [
            page.start_addr().p4_index(),
            page.start_addr().p3_index(),
            page.start_addr().p2_index(),
        ];
        let mut page_table = self.page_table();
        for index in indicies {
            let entry = page_table[index];
            page_table = Self::page_table_from_addr(entry.addr());
        }

        let entry = &mut page_table[page.start_addr().p1_index()];
        let frame = entry.frame().map_err(|err| match err {
            FrameError::FrameNotPresent => UnmapError::PageNotMapped,
            FrameError::HugePageNotSupported => UnmapError::ParentEntryHugePage,
            _ => unreachable!(),
        })?;

        entry.set_unused();

        Self::invalidate_page(page);

        Ok(frame)
    }
}
