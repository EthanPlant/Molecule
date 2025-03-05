use core::{alloc::Layout, any::Any, cell::UnsafeCell, mem, ptr::Unique};

use alloc::{alloc::alloc_zeroed, sync::Arc};

#[cfg(target_arch = "x86_64")]
use crate::arch::interrupts::apic::get_cpu_count;
#[cfg(target_arch="x86_64")]
fn get_cpu_id() -> u32 {
    use crate::arch::interrupts::apic::get_local_apic;

    get_local_apic().bsp_id() >> 24
}
pub struct PerCpu<T> {
    data: UnsafeCell<Unique<T>>,
}

impl<T> PerCpu<T> {
    pub const fn new_uninit() -> Self {
        Self {
            data: UnsafeCell::new(Unique::dangling()),
        }
    }

    pub fn new(init: fn() -> T) -> Self {
        let mut this = Self::new_uninit();
        let cpu_count = get_cpu_count();
        let size = mem::size_of::<T>() * cpu_count;

        let raw = unsafe { alloc_zeroed(Layout::from_size_align_unchecked(size, 8)).cast::<T>()};

        unsafe {
            for i in 0..cpu_count {
                raw.add(i).write(init());
            }

            this.data = UnsafeCell::new(Unique::new_unchecked(raw));
        }

        this
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        unsafe { (*self.data.get()).as_mut() }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.as_mut_ptr().offset(get_cpu_id() as isize) }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut T {
        unsafe { &mut *self.as_mut_ptr().offset(get_cpu_id() as isize) }
    }
}

pub trait Downcastable: Any + Send + Sync {
    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync>;
}

impl<T: Any + Send + Sync> Downcastable for T {
    fn as_any(self: Arc<Self>) -> Arc<dyn Any + Send + Sync> {
        self
    }
}