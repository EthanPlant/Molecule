//! Process Handling

use pid::{ProcessId, IDLE_PID};

use crate::arch::process::{switch_process, ArchProcess};

mod pid;
pub mod scheduler;

/// In-memory representation of a process.
pub struct Process {
    pid: ProcessId,
    tid: u64,

    arch_process: ArchProcess,
}

impl Process {
    /// Create a new idle process
    pub fn new_idle() -> Self {
        Self {
            pid: IDLE_PID,
            tid: 0,

            arch_process: ArchProcess::new_idle(),
        }
    }

    /// Create a new kernel process
    pub fn new_kernel(func: fn() -> !) -> Self {
        Self {
            pid: ProcessId::unique(),
            tid: 0,

            arch_process: ArchProcess::new_kernel(func),
        }
    }

    /// Switch from the current process to `next`
    pub fn switch(&self, next: &Self) {
        unsafe {
            switch_process(
                &self.arch_process as *const ArchProcess,
                &next.arch_process as *const ArchProcess,
            );
        }
    }

    pub fn get_pid(&self) -> ProcessId {
        self.pid
    }
}
