use core::alloc::Allocator;

use alloc::vec::Vec;
use bit_field::BitField;
use limine::{
    memory_map::{Entry, EntryType},
    response::MemoryMapResponse,
};

use super::addr::PhysAddr;

const FRAME_SIZE: usize = 4096;

pub unsafe trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysAddr>;
    fn deallocate_frame(&mut self, frame: PhysAddr);
}

#[derive(Debug)]
struct Bitmap<A: Allocator> {
    bitmap: Vec<usize, A>,
}

impl<A: Allocator> Bitmap<A> {
    const BLOCK_BITS: usize = core::mem::size_of::<usize>() * 8;

    pub fn new_in(alloc: A, size: usize) -> Self {
        let blocks = Self::calculate_blocks(size);
        let mut bitmap = Vec::new_in(alloc);

        bitmap.resize(blocks, 0);
        Self { bitmap }
    }

    pub fn set(&mut self, idx: usize, val: bool) {
        let (block_idx, bit_idx) = Self::get_index(idx);

        self.bitmap
            .get_mut(block_idx)
            .map(|n| n.set_bit(bit_idx, val));
    }

    pub fn is_set(&self, idx: usize) -> bool {
        let (block_idx, bit_idx) = Self::get_index(idx);
        self.bitmap[block_idx].get_bit(bit_idx)
    }

    pub fn find_first_unset(&self) -> Option<usize> {
        for (i, block) in self.bitmap.iter().enumerate() {
            let trailing_ones = block.trailing_ones();
            if trailing_ones < Self::BLOCK_BITS as u32 {
                return Some(i * Self::BLOCK_BITS + trailing_ones as usize);
            }
        }

        None
    }

    pub fn find_first_set(&self) -> Option<usize> {
        for (i, block) in self.bitmap.iter().enumerate() {
            let trailing_zeros = block.trailing_zeros();
            if trailing_zeros < Self::BLOCK_BITS as u32 {
                return Some(i * Self::BLOCK_BITS + trailing_zeros as usize);
            }
        }

        None
    }

    fn get_index(idx: usize) -> (usize, usize) {
        (idx / Self::BLOCK_BITS, idx % Self::BLOCK_BITS)
    }

    fn calculate_blocks(bits: usize) -> usize {
        if bits % Self::BLOCK_BITS == 0 {
            bits / Self::BLOCK_BITS
        } else {
            bits / Self::BLOCK_BITS + 1
        }
    }
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
