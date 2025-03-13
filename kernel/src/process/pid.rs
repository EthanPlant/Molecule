//! Process ID (PID) handling

use core::ops::Add;
use core::sync::atomic::{AtomicU64, Ordering};

pub(super) const IDLE_PID: ProcessId = ProcessId(0);

static PID_ALLOCATOR: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessId(u64);

impl ProcessId {
    /// Returns a new unused PID
    pub fn unique() -> Self {
        Self(PID_ALLOCATOR.fetch_add(1, Ordering::AcqRel))
    }
}

impl Add<u64> for ProcessId {
    type Output = ProcessId;

    fn add(self, rhs: u64) -> Self::Output {
        ProcessId(self.0 + rhs)
    }
}
