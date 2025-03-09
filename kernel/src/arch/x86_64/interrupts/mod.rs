//! x86_64 interrupt handling

mod exception;
pub(super) mod idt;

use core::arch::asm;
use core::fmt;
use core::ops::Deref;

/// Wrapper around the interrupt stack frame pushed by the CPU.
///
/// This type derefs to an [InterruptStackFrameInner] to enable reading the actual values.
///
/// This wrapper ensures that no accidental modification of the stack frame occurs, which can caused
/// undefined behaviour (see [as_mut](InterruptStackFrame::as_mut) for more information).
#[repr(C)]
pub(super) struct InterruptStackFrame {
    inner: InterruptStackFrameInner,
}

impl InterruptStackFrame {
    /// Gives mutable access to the contents of the framebuffer
    ///
    /// # Safety
    /// Modifying the content of the stack frame can easily lead to undefined behaviour. For
    /// example, by writing to the `rip` field, the CPU can jump to any arbitrary location at the
    /// end of the interrupt.
    ///
    /// As such, great care should be taken when using this function to ensure all modifications to
    /// the stack frame are sound.
    pub unsafe fn as_mut(&mut self) -> &mut InterruptStackFrameInner {
        &mut self.inner
    }
}

impl Deref for InterruptStackFrame {
    type Target = InterruptStackFrameInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl fmt::Debug for InterruptStackFrame {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.inner.fmt(f)
    }
}

#[repr(C)]
pub(super) struct InterruptStackFrameInner {
    rip: u64,
    cs: u64,
    rflags: u64,
    rsp: u64,
    ss: u64,
}

impl fmt::Debug for InterruptStackFrameInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        struct Hex(u64);
        impl fmt::Debug for Hex {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{:#x}", self.0)
            }
        }

        let mut s = f.debug_struct("InterruptStackFrame");
        s.field("rip", &self.rip);
        s.field("cs", &self.cs);
        s.field("rflags", &Hex(self.rflags));
        s.field("rsp", &self.rsp);
        s.field("ss", &self.ss);
        s.finish()
    }
}

/// Wrapper around `cli` to disable hardware interrupts.
pub fn disable_interrupts() {
    unsafe { asm!("cli", options(nomem, nostack)) };
}

/// Wrapper around `sti` to enable hardware interrupts.
pub fn enable_interrupts() {
    unsafe {
        asm!("sti", options(nomem, nostack));
    }
}

/// Check if interrupts are enabbled
pub fn are_interrupts_enabled() -> bool {
    let flags = unsafe {
        let flags: u64;
        asm!("pushf; pop {}", out(reg) flags, options(nomem, preserves_flags));
        flags
    };
    flags & (1 << 9) != 0
}
