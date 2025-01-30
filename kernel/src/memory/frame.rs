use limine::{
    memory_map::{Entry, EntryType},
    response::MemoryMapResponse,
};

use super::addr::PhysAddr;

pub unsafe trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysAddr>;
    fn deallocate_frame(&mut self, frame: PhysAddr);
}

pub struct BumpFrameAllocator {
    mem_map: &'static [&'static Entry],
    next: usize,
}

impl BumpFrameAllocator {
    pub unsafe fn init(mem_map: &'static MemoryMapResponse) -> Self {
        Self {
            mem_map: mem_map.entries(),
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysAddr> {
        self.mem_map
            .iter()
            .filter(|entry| entry.entry_type == EntryType::USABLE)
            .map(|range| range.base..range.base + range.length)
            .flat_map(|range| range.step_by(4096))
            .map(|addr| PhysAddr::new(addr as usize))
    }
}

unsafe impl FrameAllocator for BumpFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysAddr> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }

    fn deallocate_frame(&mut self, _frame: PhysAddr) {
        unimplemented!()
    }
}
