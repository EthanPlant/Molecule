use alloc::collections::btree_map::Range;
use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::ops::{Add, Bound, RangeBounds};
use core::sync::atomic::{AtomicUsize, Ordering};

use intrusive_collections::{intrusive_adapter, LinkedListLink};

use crate::arch::process::{idle_process, ArchProcess};
use crate::arch::{self};
use crate::memory::addr::VirtAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ProcessId(usize);

impl ProcessId {
    pub const fn new(pid: usize) -> Self {
        Self(pid)
    }

    fn allocate() -> Self {
        static NEXT_PID: AtomicUsize = AtomicUsize::new(0);

        Self::new(NEXT_PID.fetch_add(1, Ordering::AcqRel))
    }
}

impl Add<usize> for ProcessId {
    type Output = Self;

    fn add(self, rhs: usize) -> Self {
        Self(self.0 + rhs)
    }
}

#[derive(Debug)]
pub struct Process {
    pid: ProcessId,
    tid: ProcessId,

    arch: UnsafeCell<ArchProcess>,

    pub link: intrusive_collections::LinkedListLink,
}

impl Process {
    pub fn new_idle() -> Arc<Process> {
        Self::new_kernel(idle_process, true)
    }

    pub fn new_kernel(entry_point: fn(), enable_int: bool) -> Arc<Process> {
        let pid = ProcessId::allocate();

        let arch = ArchProcess::new_kernel(VirtAddr::new(entry_point as usize), enable_int);

        Arc::new(Process {
            pid,
            tid: pid,

            arch: UnsafeCell::new(arch),

            link: Default::default(),
        })
    }

    pub fn arch_process(&self) -> &ArchProcess {
        unsafe { &*self.arch.get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn arch_process_mut(&self) -> &mut ArchProcess {
        unsafe { &mut *self.arch.get() }
    }

    pub fn pid(&self) -> ProcessId {
        self.pid
    }
}

unsafe impl Sync for Process {}

intrusive_adapter!(pub SchedProcessAdapter = Arc<Process> : Process {link: LinkedListLink});
