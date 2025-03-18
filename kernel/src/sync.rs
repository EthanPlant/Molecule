use core::fmt::Debug;

use crate::arch::interrupts;

#[derive(Debug)]
pub struct IrqGuard {
    locked: bool,
}

impl IrqGuard {
    pub fn new() -> Self {
        let locked = interrupts::are_interrupts_enabled();
        interrupts::disable_interrupts();

        Self { locked }
    }
}

impl Drop for IrqGuard {
    fn drop(&mut self) {
        if self.locked {
            interrupts::enable_interrupts();
        }
    }
}

pub struct MutexGuard<'a, T: ?Sized + 'a> {
    guard: core::mem::ManuallyDrop<spin::MutexGuard<'a, T>>,
    irq_lock: bool,
}

impl<T: ?Sized> core::ops::Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T: ?Sized> core::ops::DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        unsafe {
            core::mem::ManuallyDrop::drop(&mut self.guard);
        }

        if self.irq_lock {
            interrupts::enable_interrupts();
        }
    }
}

pub struct Mutex<T: ?Sized> {
    inner: spin::Mutex<T>,
}

impl<T> Mutex<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: spin::Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        MutexGuard {
            guard: core::mem::ManuallyDrop::new(self.inner.lock()),
            irq_lock: false,
        }
    }

    pub fn lock_irq(&self) -> MutexGuard<T> {
        let irq_lock = interrupts::are_interrupts_enabled();

        interrupts::disable_interrupts();

        MutexGuard {
            guard: core::mem::ManuallyDrop::new(self.inner.lock()),
            irq_lock,
        }
    }
}

impl<T: Debug> Debug for Mutex<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Mutex").field("inner", &self.inner).finish()
    }
}
