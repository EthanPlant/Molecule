use core::{arch::asm, ops::Add};

use crate::memory::{
    addr::PhysAddr,
    frame::{Frame, FrameAllocator, FRAME_ALLOCATOR},
    MapError,
};

use super::{
    page_table::{self, active_level_4_table, PageTable},
    PageMap,
};

#[derive(Debug)]
pub struct AddressSpace {
    cr3: Frame,
}

impl AddressSpace {
    pub fn new() -> Result<Self, MapError> {
        let cr3 = unsafe {
            let frame = FRAME_ALLOCATOR
                .allocate_frame()
                .ok_or(MapError::AllocationFailed)?;

            let phys_addr = frame.start_addr();
            let virt_addr = phys_addr.as_hddm_virt();

            let page_table: *mut PageTable = virt_addr.as_mut_ptr();
            let page_table = &mut *page_table;

            let current_table = active_level_4_table();
            let current_table = current_table.page_table();

            for i in 0..256usize {
                page_table[i].set_unused();
            }

            for i in 256..512usize {
                page_table[i] = current_table[i];
            }

            frame
        };

        Ok(Self { cr3 })
    }

    pub fn this() -> Self {
        let cr3 = {
            let value: usize;

            unsafe {
                asm!("mov {}, cr3", out(reg) value);
            }

            let addr = PhysAddr::new(value & 0x000F_FFFF_FFFF_F000);

            Frame::containing_addr(addr)
        };

        Self { cr3 }
    }

    pub fn switch(&self) {
        let cr3 = usize::from(self.cr3.start_addr());

        unsafe {
            asm!("mov cr3, {}", in(reg) cr3);
        }
    }

    pub fn page_map(&self) -> PageMap {
        unsafe { PageMap::from_cr3(self.cr3.start_addr()) }
    }
}
