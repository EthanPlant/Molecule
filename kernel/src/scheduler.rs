use core::{clone, mem};

use alloc::{collections::btree_map::BTreeMap, sync::Arc};
use spin::Once;

use crate::{
    arch::{
        interrupts::{self, disable_interrupts},
        process::arch_switch_process,
    },
    process::{Process, ProcessId},
    sync::{Mutex, MutexGuard},
};

pub static SCHEDULER: Once<Mutex<Scheduler>> = Once::new();

pub struct Scheduler {
    processes: BTreeMap<ProcessId, Arc<Process>>,
    curr: Arc<Process>,
    idle: Arc<Process>,
}

impl Scheduler {
    pub fn new() -> Self {
        let idle = Process::new_idle();
        Self {
            processes: BTreeMap::new(),
            curr: idle.clone(),
            idle,
        }
    }

    pub fn register_process(&mut self, process: &Arc<Process>) {
        log::trace!("Registering process with pid={:?}", process.pid());
        let pid = process.pid();
        self.processes.insert(pid, process.clone());
    }

    pub fn remove_process(&mut self, pid: ProcessId) {
        self.processes.remove(&pid);
    }

    pub fn preempt(&mut self) {
        unsafe { disable_interrupts() };
        let (prev, next) = {
            let next = self.get_next_process().unwrap_or(self.idle.clone());
            if next.pid() == self.curr.pid() {
                return;
            }

            let prev = self.swap_current_process(next.clone());
            log::debug!("Switching from {:?} to {:?}", prev.pid(), next.pid());
            (prev.clone(), next)
        };

        #[cfg(target_arch = "x86_64")]
        interrupts::apic::get_local_apic().eoi();

        unsafe { SCHEDULER.get().unwrap().force_unlock() };
        arch_switch_process(prev.arch_process_mut(), next.arch_process());
    }

    fn get_next_process(&self) -> Option<Arc<Process>> {
        let curr = self.curr.pid();
        self.processes
            .range((curr + 1)..)
            .next()
            .map(|(_, proc)| proc.clone())
    }

    fn swap_current_process(&mut self, new: Arc<Process>) -> Arc<Process> {
        mem::replace(&mut self.curr, new)
    }
}

pub fn init() {
    let scheduler = Scheduler::new();
    SCHEDULER.call_once(|| Mutex::new(scheduler));
}

pub fn scheduler() -> MutexGuard<'static, Scheduler> {
    SCHEDULER
        .get()
        .expect("Scheduler is initialized")
        .lock_irq()
}
