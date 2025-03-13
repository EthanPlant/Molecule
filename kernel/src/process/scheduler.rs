use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use core::mem;

use spin::Once;

use super::pid::ProcessId;
use super::Process;
use crate::sync::{IrqGuard, Mutex, MutexGuard};

static SCHEDULER: Once<Mutex<Scheduler>> = Once::new();

pub struct Scheduler {
    processes: BTreeMap<ProcessId, Arc<Process>>,
    curr_proc: Arc<Process>,

    idle_proc: Arc<Process>,
}

impl Scheduler {
    pub fn new() -> Self {
        let idle = Arc::new(Process::new_idle());
        Self {
            processes: BTreeMap::new(),
            curr_proc: idle.clone(),

            idle_proc: idle,
        }
    }

    /// Swaps the current process for `new`
    pub fn swap_current_process(&mut self, new: Arc<Process>) -> Arc<Process> {
        mem::replace(&mut self.curr_proc, new)
    }

    /// Get the current process
    pub fn get_current_process(&self) -> Arc<Process> {
        self.curr_proc.clone()
    }

    /// Add a process to the scheduler
    pub fn add_process(&mut self, process: Process) {
        let pid = process.pid;
        let ptr = Arc::new(process);
        self.processes.insert(pid, ptr.clone());
        log::debug!("Registered process with PID {pid:?}");
    }

    /// Remove a process from the scheduler
    pub fn remove_process(&mut self, pid: ProcessId) {
        if self.processes.contains_key(&pid) {
            self.processes.remove(&pid);
        }
    }

    /// Get the next process to run
    fn get_next_process(&self) -> Option<Arc<Process>> {
        let curr_id = self.curr_proc.pid;
        self.processes
            .range((curr_id + 1)..)
            .next()
            .or_else(|| self.processes.range(..=curr_id).next())
            .map(|(_, proc)| proc.clone())
    }
}

pub fn tick() {
    let guard = IrqGuard::new();
    let (prev, next) = {
        let mut sched = get_scheduler();
        let next = sched.get_next_process().unwrap_or(sched.idle_proc.clone());
        if next.pid == sched.curr_proc.pid {
            return;
        }
        let prev = sched.swap_current_process(next.clone());
        (prev, next)
    };
    core::mem::drop(guard);
    prev.switch(&next);
}

pub fn init() {
    SCHEDULER.call_once(|| Mutex::new(Scheduler::new()));
}

pub fn get_scheduler() -> MutexGuard<'static, Scheduler> {
    SCHEDULER
        .get()
        .expect("Attempted to get scheduler before it was initialized")
        .lock_irq()
}
