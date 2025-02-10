use page_table::PageTable;

use crate::memory::{
    addr::{PhysAddr, VirtAddr, HHDM_OFFSET},
    frame::{Frame, FrameError},
    page::Page,
    PageSize4K, VirtualMemoryManager,
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
    page_table::init(mem_map);
}

#[derive(Debug)]
pub struct PageMap {
    pml4: PhysAddr,
}

impl PageMap {
    unsafe fn from_cr3(cr3: PhysAddr) -> Self {
        Self { pml4: cr3 }
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

    fn translate_page(&self, page: Page<PageSize4K>) -> Result<Frame, FrameError> {
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
}
