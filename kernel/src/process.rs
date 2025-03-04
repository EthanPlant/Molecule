use core::{
    cell::UnsafeCell,
    sync::atomic::{AtomicUsize, Ordering},
};

use alloc::sync::Arc;

use crate::{
    arch::{self, process::ArchProcess},
    memory::addr::VirtAddr,
};

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct ProcessId(usize);

impl ProcessId {
    pub const fn new(pid: usize) -> Self {
        Self(pid)
    }

    fn allocate() -> Self {
        static NEXT_PID: AtomicUsize = AtomicUsize::new(1);

        Self::new(NEXT_PID.fetch_add(1, Ordering::AcqRel))
    }
}

#[derive(Debug)]
pub struct Process {
    pid: ProcessId,
    tid: ProcessId,

    pub arch: UnsafeCell<ArchProcess>,
}

impl Process {
    pub fn new_idle() -> Arc<Process> {
        let pid = ProcessId::allocate();

        let arch = ArchProcess::new_idle();

        Arc::new(Process {
            pid,
            tid: pid,

            arch: UnsafeCell::new(arch),
        })
    }

    pub fn new_kernel(entry_point: fn(), enable_int: bool) -> Arc<Process> {
        let pid = ProcessId::allocate();

        let arch = ArchProcess::new_kernel(VirtAddr::new(entry_point as usize), enable_int);

        Arc::new(Process {
            pid,
            tid: pid,

            arch: UnsafeCell::new(arch),
        })
    }
}

unsafe impl Sync for Process {}
