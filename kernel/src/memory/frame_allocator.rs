//! Physical Frame Allocation

use alloc::alloc::Allocator;
use alloc::vec::Vec;
use core::cmp::min;
use core::mem::{self, MaybeUninit};

use bit_field::BitField;
use limine::memory_map::EntryType;
use spin::Once;

use super::addr::{align_up, PhysAddr};
use super::bootstrap_alloc::{BootstrapAlloc, BootstrapAllocRef};
use super::frame::PhysFrame;
use super::mem_map::{MemoryRegion, MemoryRegionIter, MemoryRegionType};
use super::page::{PageSize, Size2M, Size4K};
use crate::sync::Mutex;

static FRAME_ALLOCATOR: Once<LockedFrameAllocator> = Once::new();

const BUDDY_SIZES: [usize; 10] = [
    Size4K::SIZE,
    Size4K::SIZE * 2,
    Size4K::SIZE * 4,
    Size4K::SIZE * 8,
    Size4K::SIZE * 16,
    Size4K::SIZE * 32,
    Size4K::SIZE * 64,
    Size4K::SIZE * 128,
    Size4K::SIZE * 256,
    Size4K::SIZE * 512,
];

/// A trait for types that can allocate or deallocate frames of memory
///
/// # Safety
///
/// The implementer must ensure that `allocate_frame` only returns free, unused frames.
pub unsafe trait FrameAllocator<S: PageSize> {
    /// Allcoate a frame of the appropriate size and return it if possible.
    fn allocate_frame(&self) -> Option<PhysFrame<S>>;

    /// Deallocate the given frame.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the passed frame is unused.
    #[allow(dead_code)]
    unsafe fn deallocate_frame(&self, frame: PhysFrame<S>);
}

/// A locked global frame allocator.
pub struct LockedFrameAllocator(Mutex<BuddyAllocator>);

impl LockedFrameAllocator {
    /// Create a new instance of the locked frame allocator.
    pub fn new(mem_map: &mut limine::response::MemoryMapResponse) -> Self {
        let inner = BuddyAllocator::new(mem_map);

        Self(Mutex::new(inner))
    }

    /// Get the total amount of memory available to the frame allocator.
    pub fn get_total_memory(&self) -> usize {
        self.0.lock().size
    }

    /// Allocate `bytes` returning the address of the allocation. Returns `None` if the allocation
    /// failed.
    pub fn alloc(&self, bytes: usize) -> Option<PhysAddr> {
        let order = Self::order_from_size(bytes);

        let addr = self.0.lock_irq().allocate(order);
        addr
    }

    /// Deallocate `bytes` starting at the given address.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the entire range from `addr` to `addr + size` is unused.
    pub unsafe fn dealloc(&self, addr: PhysAddr, bytes: usize) {
        let order = Self::order_from_size(bytes);

        self.0.lock_irq().deallocate(addr, order);
    }

    /// Allocate `bytes` and fill it with zeroes, returning the address of the allocation. Returns
    /// `None` if the allocation failed.
    pub fn alloc_zeroed(&self, bytes: usize) -> Option<PhysAddr> {
        let addr = self.alloc(bytes)?;
        let virt = addr.as_hhdm_virt();
        let slice = virt.as_bytes_mut(bytes);
        if slice.is_err() {
            log::warn!(
                "PFA: Attempted to zero slice of {bytes} starting at {:x} but recieved error {:?}",
                addr,
                slice
            );
            // Safety: Address was just allocated and is currently unused
            unsafe { self.dealloc(addr, bytes) };
            return None;
        }

        let slice = slice.unwrap();
        slice.fill(0);

        Some(addr)
    }

    /// Find the smallest order for a given size.
    fn order_from_size(size: usize) -> usize {
        BUDDY_SIZES
            .iter()
            .enumerate()
            .find(|(_, &buddy_size)| buddy_size >= size)
            .expect("Buddy is large enough to fit")
            .0
    }
}

unsafe impl FrameAllocator<Size4K> for LockedFrameAllocator {
    fn allocate_frame(&self) -> Option<PhysFrame<Size4K>> {
        let phys = self.alloc(Size4K::SIZE)?;
        Some(PhysFrame::containing_addr(phys))
    }

    unsafe fn deallocate_frame(&self, frame: PhysFrame<Size4K>) {
        self.0
            .lock_irq()
            .deallocate(frame.start_addr(), Self::order_from_size(Size4K::SIZE));
    }
}

unsafe impl FrameAllocator<Size2M> for LockedFrameAllocator {
    fn allocate_frame(&self) -> Option<PhysFrame<Size2M>> {
        let phys = self.alloc(Size2M::SIZE)?;
        Some(PhysFrame::containing_addr(phys))
    }

    unsafe fn deallocate_frame(&self, frame: PhysFrame<Size2M>) {
        self.0
            .lock_irq()
            .deallocate(frame.start_addr(), Self::order_from_size(Size2M::SIZE));
    }
}

/// Bitmap used by the buddy allocator
struct Bitmap<A: Allocator> {
    bitmap: Vec<usize, A>,
}

impl<A: Allocator> Bitmap<A> {
    /// The number of bits in a single block of the bitmap.
    const BLOCK_BITS: usize = mem::size_of::<usize>() * 8;

    /// Create an empty bitmap.
    pub const fn empty(alloc: A) -> Self {
        Self {
            bitmap: Vec::new_in(alloc),
        }
    }

    /// Create a new bitmap with `size` bits
    pub fn new(alloc: A, size: usize) -> Self {
        let blocks = Self::calculate_blocks(size);
        let mut bitmap = Vec::new_in(alloc);
        bitmap.resize(blocks, 0);

        Self { bitmap }
    }

    /// Set the bit at `idx` with `val`.
    pub fn set(&mut self, idx: usize, val: bool) {
        let (block_idx, bit_idx) = Self::get_index(idx);
        self.bitmap
            .get_mut(block_idx)
            .map(|n| n.set_bit(bit_idx, val));
    }

    /// Check if the bit at `idx` is set.
    pub fn is_set(&self, idx: usize) -> bool {
        let (block_idx, bit_idx) = Self::get_index(idx);
        self.bitmap[block_idx].get_bit(bit_idx)
    }

    /// Find the first set bit of the bitmap, returning None if no bits are set.
    pub fn find_first_set(&self) -> Option<usize> {
        for (i, block) in self.bitmap.iter().enumerate() {
            let trailing_zeroes = block.trailing_zeros();
            if trailing_zeroes < Self::BLOCK_BITS as u32 {
                return Some(i * Self::BLOCK_BITS + trailing_zeroes as usize);
            }
        }

        None
    }

    /// Calculate the block and offset from an index.
    fn get_index(idx: usize) -> (usize, usize) {
        (idx / Self::BLOCK_BITS, idx % Self::BLOCK_BITS)
    }

    /// Returns the number of blocks needed for a bitmap of of length `bits`.
    fn calculate_blocks(bits: usize) -> usize {
        if bits % Self::BLOCK_BITS == 0 {
            bits / Self::BLOCK_BITS
        } else {
            bits / Self::BLOCK_BITS + 1
        }
    }
}

/// A buddy allocator used to allocate physical frames in memory
struct BuddyAllocator {
    buddies: [Bitmap<BootstrapAllocRef>; 10],
    free: [usize; 10],
    base: PhysAddr,
    end: PhysAddr,
    size: usize,
}

impl BuddyAllocator {
    /// Construct a new buddy allocator from a Limine memory map
    pub fn new(mem_map_resp: &mut limine::response::MemoryMapResponse) -> Self {
        let free_entries = mem_map_resp
            .entries()
            .iter()
            .filter(|entry| entry.entry_type == EntryType::USABLE)
            .count();
        let mem_map = mem_map_resp.entries_mut();
        let requested_size = align_up(mem::size_of::<MemoryRegion>() * free_entries, Size4K::SIZE);

        let entry = mem_map
            .iter_mut()
            .find(|entry| {
                entry.entry_type == EntryType::USABLE && entry.length as usize >= requested_size
            })
            .expect("PFA: Failed to find a suitable region for the memory map");

        let region = PhysAddr::new(entry.base as usize);
        entry.base += requested_size as u64;
        entry.length -= requested_size as u64;

        let free_entries = mem_map_resp
            .entries()
            .iter()
            .filter(|entry| entry.entry_type == EntryType::USABLE)
            .count();

        let mut iter = mem_map_resp.entries().iter();
        let cursor = iter
            .find(|entry| entry.entry_type == EntryType::USABLE)
            .expect("At least one free area of memory exists");

        // Safety: `region` is large enough to contain all of our memory regions.
        let regions = unsafe {
            let virt_addr = region.as_hhdm_virt();

            let regions = core::slice::from_raw_parts_mut::<MaybeUninit<MemoryRegion>>(
                virt_addr.as_mut_ptr(),
                free_entries,
            );

            let region_iter = MemoryRegionIter::new(
                iter,
                PhysAddr::new(cursor.base as usize),
                PhysAddr::new(cursor.base as usize + cursor.length as usize),
            );

            for (i, region) in region_iter.enumerate() {
                log::debug!("PFA: Free memory region {:?}", region);
                regions[i] = MaybeUninit::new(region);
            }

            MaybeUninit::slice_assume_init_mut(regions)
        };

        let base = regions[0].base;
        let end = regions[free_entries - 1].base + regions[free_entries - 1].size;

        let bootstrap = BootstrapAlloc::new(&mut regions[..free_entries]);
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
            size: 0,
        };

        let size = this.end - this.base;

        for (i, buddy_size) in BUDDY_SIZES.iter().enumerate() {
            let chunk = size / buddy_size;
            this.buddies[i] = Bitmap::new(bootstrap_ref, chunk);
        }

        for region in bootstrap_ref.get_inner().memory_regions.lock().iter() {
            if region.region_type == MemoryRegionType::Free {
                this.insert_range(region.base, region.base + region.size);
                this.size += region.size;
            }
        }

        log::info!("{} MiB free memory", this.size / 1024 / 1024);

        this
    }

    /// Allocate a region with a given order, returning the physical address of the allocation. This
    /// function returns `None` if the allocation failed.
    fn allocate(&mut self, order: usize) -> Option<PhysAddr> {
        let size = BUDDY_SIZES[order];

        for (i, &buddy_size) in BUDDY_SIZES[order..].iter().enumerate() {
            let i = i + order;

            if self.free[i] > 0 {
                let result = self.find_free(i)?;
                let mut remaining = buddy_size - size;

                if remaining > 0 {
                    for j in (0..=i).rev() {
                        let b = BUDDY_SIZES[j];

                        while remaining >= b {
                            self.set_chunk_free(result + (remaining - b) + size, j);
                            remaining -= b;
                        }
                    }
                }
                return Some(result);
            }
        }

        None
    }

    /// Deallocate a region starting at `addr` with a given order.
    ///
    /// # Safety
    ///
    /// Caller must ensure the region is unused.
    unsafe fn deallocate(&mut self, mut addr: PhysAddr, mut order: usize) {
        while order < BUDDY_SIZES.len() {
            if order < BUDDY_SIZES.len() - 1 {
                let buddy = self.get_buddy(addr, order);

                if self.set_chunk_used(buddy, order) {
                    addr = min(addr, buddy);
                    order += 1;
                } else {
                    self.set_chunk_free(addr, order);
                    break;
                }
            } else {
                self.set_chunk_free(addr, order);
                break;
            }
        }
    }

    /// Insert a region of free memory
    fn insert_range(&mut self, base: PhysAddr, end: PhysAddr) {
        let mut remaining = end - base;
        let mut current = base;

        while remaining > 0 {
            let order = Self::find_order(current, remaining);
            let size = BUDDY_SIZES[order];
            self.set_chunk_free(current, order);

            current += size;
            remaining -= size;
        }
    }

    /// Set a region of memory as free, returning whether or not the region changed
    fn set_chunk_free(&mut self, addr: PhysAddr, order: usize) -> bool {
        let idx = self.get_region_index(addr, order);

        let buddy = &mut self.buddies[order];
        let change = !buddy.is_set(idx);

        if change {
            buddy.set(idx, true);
            self.free[order] += 1;
        }

        change
    }

    /// Set a region of memory as used, returning whether or not the region changed
    fn set_chunk_used(&mut self, addr: PhysAddr, order: usize) -> bool {
        let idx = self.get_region_index(addr, order);

        let buddy = &mut self.buddies[order];
        let change = buddy.is_set(idx);

        if change {
            buddy.set(idx, false);
            self.free[order] -= 1;
        }

        change
    }

    /// Get the buddy of the region starting at `addr` of a given order.
    fn get_buddy(&self, addr: PhysAddr, order: usize) -> PhysAddr {
        let size = BUDDY_SIZES[order];
        let base = addr.align_down(size * 2);

        if base == addr {
            addr + size
        } else {
            base
        }
    }

    /// Get the index for a region of memory
    fn get_region_index(&self, addr: PhysAddr, order: usize) -> usize {
        let offset = addr - self.base;
        offset / BUDDY_SIZES[order]
    }

    /// Finds a free chunk with the provided order and marks it as used. Returns `None` if no free
    /// chunks exist.
    fn find_free(&mut self, order: usize) -> Option<PhysAddr> {
        let buddy = &mut self.buddies[order];
        let first_free = buddy.find_first_set()?;

        buddy.set(first_free, false);
        self.free[order] -= 1;

        Some(self.base.align_up(BUDDY_SIZES[order]) + (BUDDY_SIZES[order] * first_free))
    }

    /// Find the best order to insert this chunk into
    fn find_order(addr: PhysAddr, chunk_size: usize) -> usize {
        for order in (0..BUDDY_SIZES.len()).rev() {
            let size = BUDDY_SIZES[order];
            if size > chunk_size {
                continue;
            }
            let mask = size - 1;
            if mask & addr.as_usize() != 0 {
                continue;
            }

            return order;
        }

        0
    }
}

/// Initialize the frame allocator with the memory map from Limine.
pub fn init(mem_map: &mut limine::response::MemoryMapResponse) {
    log::info!("PFA: Initializing frame allocator");
    FRAME_ALLOCATOR.call_once(|| LockedFrameAllocator::new(mem_map));
    log::info!("PFA: Frame allocator initialized");
}

/// Get the global frame allocator.
pub fn get_frame_allocator() -> &'static LockedFrameAllocator {
    FRAME_ALLOCATOR
        .get()
        .expect("Attempted to get frame allocator before it was initialized")
}
