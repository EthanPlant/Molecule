use core::{
    arch::asm,
    fmt::Debug,
    ops::{Index, IndexMut},
};

use bitflags::bitflags;

use crate::memory::{
    addr::PhysAddr,
    frame::{Frame, FrameError, FRAME_ALLOCATOR},
    PageSize, PageSize4K,
};

bitflags! {
    #[derive(Debug, Copy, Clone)]
    pub struct PageTableFlags: u64 {
        const PRESENT = 1;
        const WRITEABLE = 1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const NO_CACHE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const HUGE_PAGE = 1 << 7;
        const GLOBAL = 1 << 8;
        const NO_EXECUTE = 1 << 63;
    }
}

#[derive(Clone, Default, Copy)]
#[repr(transparent)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    const ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;
    const FLAGS_MASK: u64 = 0x8000_0000_0000_01FF;

    pub const fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn set_unused(&mut self) {
        self.0 = 0;
    }

    pub const fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.0)
    }

    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new((self.0 & Self::ADDR_MASK) as usize)
    }

    pub fn frame(&self) -> Result<Frame, FrameError> {
        if !self.flags().contains(PageTableFlags::PRESENT) {
            return Err(FrameError::FrameNotPresent);
        } else if self.flags().contains(PageTableFlags::HUGE_PAGE) {
            return Err(FrameError::HugePageNotSupported);
        }
        Ok(Frame::containing_addr(self.addr()))
    }

    pub fn set_addr(&mut self, addr: PhysAddr, flags: PageTableFlags) {
        assert!(addr.is_aligned(PageSize4K::SIZE));

        self.0 &= !Self::ADDR_MASK;
        self.0 |= usize::from(addr) as u64;

        self.set_flags(flags)
    }

    pub fn set_frame(&mut self, frame: Frame, flags: PageTableFlags) {
        assert!(!flags.contains(PageTableFlags::HUGE_PAGE));
        self.set_addr(frame.start_addr(), flags);
    }

    pub fn set_flags(&mut self, flags: PageTableFlags) {
        self.0 &= !Self::FLAGS_MASK;
        self.0 |= flags.bits();
    }
}

impl Debug for PageTableEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut f = f.debug_struct("PageTableEntry");
        f.field("addr", &self.addr());
        f.field("flags", &self.flags());
        f.finish()
    }
}

const ENTRY_COUNT: usize = 512;

#[repr(align(4096))]
#[repr(C)]
#[derive(Clone, Debug)]
pub struct PageTable {
    entries: [PageTableEntry; ENTRY_COUNT],
}

impl PageTable {
    pub fn zero(&mut self) {
        for entry in &mut self.entries {
            entry.set_unused();
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self {
            entries: [PageTableEntry::default(); ENTRY_COUNT],
        }
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        &self.entries[index]
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.entries[index]
    }
}

pub unsafe fn active_level_4_table() -> &'static mut PageTable {
    let table_ptr = unsafe {
        let table_ptr: usize;
        asm!("mov rax, cr3", out("rax") table_ptr);
        table_ptr
    };
    let page_table_addr = PhysAddr::new(table_ptr).as_hddm_virt();
    let page_table: *mut PageTable = page_table_addr.as_mut_ptr();

    &mut *page_table
}

pub fn init(mem_map: &mut limine::response::MemoryMapResponse) {
    let level_4_table = unsafe { active_level_4_table() };
    FRAME_ALLOCATOR.init(mem_map);
    for (i, entry) in level_4_table.iter().enumerate() {
        if !entry.is_unused() {
            log::debug!("L4 Entry {i}: {:x?}", entry);
        }
    }
}
