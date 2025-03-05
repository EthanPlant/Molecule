use core::arch;

use alloc::{collections::{btree_map::BTreeMap}, sync::Arc};
use intrusive_collections::LinkedList;
use spin::Once;

use crate::{arch::{interrupts::apic::get_local_apic, process::{self, arch_switch_process}}, process::{Process, ProcessId, SchedProcessAdapter}, sync::{IrqGuard, Mutex}, utils::PerCpu};
use crate::utils::Downcastable;

struct ProcessList(Mutex<BTreeMap<ProcessId, Arc<Process>>>);

static SCHEDULER: Once<Scheduler> = Once::new();

pub trait SchedulerInterface: Send + Sync + Downcastable {
    fn register_process(&self, process: Arc<Process>);

    fn current_process(&self) -> Option<Arc<Process>>;

    fn init(&self);

    fn preempt(&self);
}

impl ProcessList {
    fn new() -> Self {
        Self(Mutex::new(BTreeMap::new()))
    }

    fn register_process(&self, process_id: ProcessId, process: Arc<Process>) {
        self.0.lock_irq().insert(process_id, process);
    }

    fn remove_task(&self, process_id: ProcessId) {
        self.0.lock_irq().remove(&process_id);
    }
}

unsafe impl Send for ProcessList {}
unsafe impl Sync for ProcessList {}

pub struct Scheduler {
    processes: ProcessList,
    pub inner: Arc<dyn SchedulerInterface>,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            processes: ProcessList::new(),
            inner: RoundRobin::new(),
        }
    }

    pub fn register_process(&self, process: Arc<Process>) {
        log::debug!("Registering process with PID {:?}", process.pid());
        self.processes.register_process(process.pid(), process.clone());
        self.inner.register_process(process);
    }

    pub fn current_process(&self) -> Option<Arc<Process>> {
        self.inner.current_process()
    }
}

struct ProcessQueue {
    idle: Arc<Process>,
    current: Option<Arc<Process>>,
    preempt: Arc<Process>,

    processes: LinkedList<SchedProcessAdapter>,
}

impl ProcessQueue {
    fn new() -> Self {
        Self {
            idle: Process::new_idle(),
            preempt: Process::new_kernel(preempter, false),
            current: None,

            processes: LinkedList::new(SchedProcessAdapter::new()),
        }
    }

    fn push_process(&mut self, process: Arc<Process>) {
        debug_assert!(!process.link.is_linked());

        self.processes.push_back(process);
    }
}

pub struct RoundRobin {
    queue: PerCpu<ProcessQueue>,
}

impl RoundRobin {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            queue: PerCpu::new(ProcessQueue::new),
        })
    }

    pub fn next_process(&self) {
        let guard = IrqGuard::new();
        let queue = self.queue.get_mut();

        if let Some(process) = queue.processes.pop_front() {
            if let Some(current) = queue.current.clone() {
                if !current.link.is_linked() && current.pid() != process.pid() {
                    queue.push_process(current);
                }
            }

            queue.current = Some(process.clone());
            core::mem::drop(guard);
            arch_switch_process(queue.preempt.arch_process_mut(), process.arch_process());
        } else {
            if let Some(current) = queue.current.as_ref() {
                core::mem::drop(guard);
                arch_switch_process(queue.preempt.arch_process_mut(), current.arch_process());
                return;
            }

            queue.current = None;
            core::mem::drop(guard);
            arch_switch_process(queue.preempt.arch_process_mut(), queue.idle.arch_process());
        }
    }
}

impl SchedulerInterface for RoundRobin {
    fn register_process(&self, process: Arc<Process>) {
        log::debug!("Registering process {:?} on CPU {}", process.pid(), get_local_apic().bsp_id() >> 24);
        let queue = self.queue.get_mut();
        queue.push_process(process);
    }

    fn current_process(&self) -> Option<Arc<Process>> {
        self.queue.get().current.clone()
    }

    fn init(&self) {}

    fn preempt(&self) {
        let guard = IrqGuard::new();
        let queue = self.queue.get();

        if let Some(current) = queue.current.as_ref() {
            core::mem::drop(guard);
            arch_switch_process(current.arch_process_mut(), queue.preempt.arch_process());
        } else {
            core::mem::drop(guard);
            arch_switch_process(queue.idle.arch_process_mut(), queue.preempt.arch_process());
        }
    }
}

unsafe impl Send for RoundRobin {}
unsafe impl Sync for RoundRobin {}

pub fn scheduler() -> &'static Scheduler {
    SCHEDULER.get().expect("Attempted to get the scheduler before it was initialized")
}

pub fn current_process() -> Option<Arc<Process>> {
    scheduler().current_process()
}

pub fn is_init() -> bool {
    SCHEDULER.is_completed()
}

pub fn init() {
    SCHEDULER.call_once(Scheduler::new).inner.init();
}

fn preempter() {
    let scheduler_ref = scheduler()
        .inner
        .clone()
        .as_any()
        .downcast::<RoundRobin>()
        .ok()
        .unwrap();

    loop {
        scheduler_ref.next_process();
    }
}