use core::alloc::Allocator;

use alloc::vec::Vec;
use bit_field::BitField;
use limine::{
    memory_map::{Entry, EntryType},
    response::MemoryMapResponse,
};
use spin::mutex::Mutex;

use crate::memory::{
    addr::align_up,
    bootstrap::{self, BootstrapAlloc},
    memmap::{MemoryRegionIter, MemoryRegionType},
};

use super::{addr::PhysAddr, bootstrap::BootstrapAllocRef, memmap::MemoryRegion};

pub static FRAME_ALLOCATOR: LockedFrameAllocator = LockedFrameAllocator::new();

const FRAME_SIZE: usize = 4096;

const BUDDY_SIZE: [usize; 10] = [
    FRAME_SIZE,
    FRAME_SIZE * 2,
    FRAME_SIZE * 4,
    FRAME_SIZE * 8,
    FRAME_SIZE * 16,
    FRAME_SIZE * 32,
    FRAME_SIZE * 64,
    FRAME_SIZE * 128,
    FRAME_SIZE * 256,
    FRAME_SIZE * 512,
];

pub unsafe trait FrameAllocator {
    fn allocate_frame(&self) -> Option<PhysAddr>;
    fn deallocate_frame(&self, frame: PhysAddr);
}

pub struct LockedFrameAllocator(Mutex<BuddyFrameAllocator>);

impl LockedFrameAllocator {
    pub const fn new() -> Self {
        let bootstrap_ref = BootstrapAllocRef {
            inner: core::ptr::null(),
        };

        Self(Mutex::new(BuddyFrameAllocator {
            buddies: [
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
            ],
            free: [0; 10],
            base: PhysAddr::null(),
            end: PhysAddr::null(),
        }))
    }

    pub fn init(&self, mem_map: &mut limine::response::MemoryMapResponse) {
        let mut allocator = self.0.lock();
        *allocator = BuddyFrameAllocator::new(mem_map);
    }

    pub fn alloc(&self, size: usize) -> Option<PhysAddr> {
        let order = Self::order_from_size(size);
        self.0.lock().allocate_frame(order)
    }

    pub fn dealloc(&self, addr: PhysAddr, size: usize) {
        let order = Self::order_from_size(size);
        self.0.lock().deallocate_frame(addr, order);
    }

    fn order_from_size(size: usize) -> usize {
        for (i, &buddy_size) in BUDDY_SIZE.iter().enumerate() {
            if buddy_size >= size {
                return i;
            }
        }

        unreachable!();
    }
}

unsafe impl FrameAllocator for LockedFrameAllocator {
    fn allocate_frame(&self) -> Option<PhysAddr> {
        self.alloc(FRAME_SIZE)
    }

    fn deallocate_frame(&self, frame: PhysAddr) {
        self.dealloc(frame, FRAME_SIZE);
    }
}

#[derive(Debug)]
struct Bitmap<A: Allocator> {
    bitmap: Vec<usize, A>,
}

impl<A: Allocator> Bitmap<A> {
    const BLOCK_BITS: usize = core::mem::size_of::<usize>() * 8;

    pub const fn empty(alloc: A) -> Self {
        Self {
            bitmap: Vec::new_in(alloc),
        }
    }

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

#[derive(Debug)]
pub struct BuddyFrameAllocator {
    buddies: [Bitmap<BootstrapAllocRef>; 10],
    free: [usize; 10],
    base: PhysAddr,
    end: PhysAddr,
}

impl BuddyFrameAllocator {
    pub fn new(memory_map_resp: &mut limine::response::MemoryMapResponse) -> Self {
        let mem_map = memory_map_resp.entries_mut();
        let requested_size = align_up(
            core::mem::size_of::<MemoryRegion>() * mem_map.len(),
            FRAME_SIZE,
        );

        let entry = mem_map
            .iter_mut()
            .find(|entry| {
                entry.entry_type == EntryType::USABLE && entry.length as usize >= requested_size
            })
            .expect("Didn't find suitable region for memory map");

        let region = PhysAddr::new(entry.base as usize);
        entry.base += requested_size as u64;
        entry.length -= requested_size as u64;

        let mut iter = memory_map_resp.entries().iter();
        let cursor = iter.next().expect("Unexpected end of memory map");

        let regions = unsafe {
            let virt_addr = region.as_hddm_virt();

            core::slice::from_raw_parts_mut::<MemoryRegion>(virt_addr.as_mut_ptr(), requested_size)
        };

        let region_iter = MemoryRegionIter {
            iter,
            cursor_base: PhysAddr::new(cursor.base as usize),
            cursor_end: PhysAddr::new(cursor.base as usize + cursor.length as usize),
        };

        let mut i = 0;
        for region in region_iter {
            regions[i] = region;
            i += 1;
        }

        let base = regions[0].base;
        let end = regions[i - 1].base + regions[i - 1].size;

        let bootstrap = BootstrapAlloc::new(&mut regions[..i]);
        let bootstrap_ref = BootstrapAllocRef::new(&bootstrap);

        let mut this = Self {
            buddies: [
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
                Bitmap::empty(bootstrap_ref),
            ],
            free: [0; 10],
            base,
            end,
        };

        let size = this.end - this.base;

        log::info!("Memory size: {} frames", size / 4096);

        for (i, bsize) in BUDDY_SIZE.iter().enumerate() {
            let chunk = size / bsize;
            this.buddies[i] = Bitmap::new_in(bootstrap_ref, chunk);
        }

        for region in bootstrap_ref.get_inner().memory_ranges.lock().iter() {
            if region.region_type == MemoryRegionType::Free {
                this.insert_range(region.base, region.base + region.size);
            }
        }

        this
    }

    fn allocate_frame(&mut self, order: usize) -> Option<PhysAddr> {
        let size = BUDDY_SIZE[order];

        for (i, &buddy_size) in BUDDY_SIZE[order..].iter().enumerate() {
            let i = i + order;
            if self.free[i] > 0 {
                let result = self.find_free(i)?;
                let mut remaining = buddy_size - size;

                if remaining > 0 {
                    for j in (0..=i).rev() {
                        let b = BUDDY_SIZE[j];
                        while remaining >= b {
                            self.set_bit(result + (remaining - b), j);
                            remaining -= size;
                        }
                    }
                }

                return Some(result);
            }
        }

        None
    }

    fn deallocate_frame(&mut self, mut addr: PhysAddr, mut order: usize) {
        while order < BUDDY_SIZE.len() {
            if order < BUDDY_SIZE.len() - 1 {
                let buddy = self.get_buddy(addr, order);
                if self.clear_bit(buddy, order) {
                    addr = core::cmp::min(addr, buddy);
                    order += 1;
                } else {
                    self.set_bit(addr, order);
                    break;
                }
            } else {
                self.set_bit(addr, order);
                break;
            }
        }
    }

    fn get_buddy(&self, addr: PhysAddr, order: usize) -> PhysAddr {
        let size = BUDDY_SIZE[order];
        let base = addr.align_down(size * 2);

        if base == addr {
            addr + size
        } else {
            base
        }
    }

    fn find_free(&mut self, order: usize) -> Option<PhysAddr> {
        let buddy = &mut self.buddies[order];
        let first_free = buddy.find_first_set()?;
        buddy.set(first_free, false);
        self.free[order] -= 1;

        Some(self.base.align_up(BUDDY_SIZE[order]) + (BUDDY_SIZE[order] * first_free))
    }

    fn insert_range(&mut self, base: PhysAddr, end: PhysAddr) {
        let mut remaining = end - base;
        let mut current = base;

        while remaining > 0 {
            let order = Self::find_order(current, remaining);
            let size = BUDDY_SIZE[order];
            self.set_bit(current, order);

            current += size;
            remaining -= size;
        }
    }

    fn set_bit(&mut self, addr: PhysAddr, order: usize) -> bool {
        let idx = self.get_bit_idx(addr, order);
        let buddy = &mut self.buddies[order];
        let change = !buddy.is_set(idx);

        if change {
            buddy.set(idx, true);
            self.free[order] += 1;
        }

        change
    }

    fn clear_bit(&mut self, addr: PhysAddr, order: usize) -> bool {
        let idx = self.get_bit_idx(addr, order);
        let buddy = &mut self.buddies[order];
        let change = buddy.is_set(idx);

        if change {
            buddy.set(idx, false);
            self.free[order] -= 1;
        }

        change
    }

    fn get_bit_idx(&self, addr: PhysAddr, order: usize) -> usize {
        let offset = addr - self.base;
        offset / BUDDY_SIZE[order]
    }

    fn find_order(addr: PhysAddr, chunk_size: usize) -> usize {
        for order in (0..BUDDY_SIZE.len()).rev() {
            let size = BUDDY_SIZE[order];
            if size > chunk_size {
                continue;
            }
            let mask = size - 1;
            if mask & usize::from(addr) != 0 {
                continue;
            }

            return order;
        }

        0
    }
}

// pub struct BumpFrameAllocator {
//     mem_map: &'static [&'static Entry],
//     next: usize,
// }

// impl BumpFrameAllocator {
//     pub unsafe fn init(mem_map: &'static MemoryMapResponse) -> Self {
//         Self {
//             mem_map: mem_map.entries(),
//             next: 0,
//         }
//     }

//     fn usable_frames(&self) -> impl Iterator<Item = PhysAddr> {
//         self.mem_map
//             .iter()
//             .filter(|entry| entry.entry_type == EntryType::USABLE)
//             .map(|range| range.base..range.base + range.length)
//             .flat_map(|range| range.step_by(4096))
//             .map(|addr| PhysAddr::new(addr as usize))
//     }
// }

// unsafe impl FrameAllocator for BumpFrameAllocator {
//     fn allocate_frame(&mut self) -> Option<PhysAddr> {
//         let frame = self.usable_frames().nth(self.next);
//         self.next += 1;
//         frame
//     }

//     fn deallocate_frame(&mut self, _frame: PhysAddr) {
//         unimplemented!()
//     }
// }
