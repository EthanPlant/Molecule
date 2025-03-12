//! Architecture specific process information.

use core::arch::{asm, global_asm};
use core::mem::offset_of;

use super::interrupts::{enable_interrupts, InterruptStackFrame, InterruptStackFrameInner};
use super::memory::address_space::AddressSpace;
use crate::memory::addr::{PhysAddr, VirtAddr};
use crate::memory::frame_allocator::get_frame_allocator;
use crate::process::Process;

const KERNEL_STACK_SIZE: usize = 1024 * 16;

/// Wrapper for the kernel stack
struct KernelStack(VirtAddr);

impl KernelStack {
    /// Allocate a new kernel stack
    pub fn new() -> Self {
        let stack = get_frame_allocator()
            .alloc_zeroed(KERNEL_STACK_SIZE)
            .expect("Space available for kernel stack");
        // Safety: addr is non-null.
        Self(stack.as_hhdm_virt())
    }

    /// Returns a pointer to the top of the stack
    pub fn top(&self) -> VirtAddr {
        self.0 + KERNEL_STACK_SIZE
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let phys = AddressSpace::this()
            .translate_addr(self.0)
            .expect("Kernel stack is mapped");
        unsafe {
            get_frame_allocator().dealloc(phys, KERNEL_STACK_SIZE);
        }
    }
}

/// Architecture specific process information
pub struct ArchProcess {
    addr_space: AddressSpace,
    kernel_stack: KernelStack,
    kernel_sp: VirtAddr,
}

impl ArchProcess {
    /// Create a new idle process.
    pub fn new_idle() -> Self {
        let stack: KernelStack = KernelStack::new();
        let kernel_sp = stack.top();
        // Safety: `kernel_sp` is a valid stack pointer
        let frame = unsafe { init_idle(kernel_sp) };
        Self {
            addr_space: AddressSpace::this(),
            kernel_stack: stack,
            kernel_sp: frame,
        }
    }

    /// Create a kernel process.
    pub fn new_kernel(func: fn() -> !) -> Self {
        let stack = KernelStack::new();
        let kernel_sp = stack.top();
        // Safety: `kernel_sp` is a valid stack pointer
        let frame = unsafe { init_kernel(kernel_sp, func) };
        Self {
            addr_space: AddressSpace::this(),
            kernel_stack: stack,
            kernel_sp: frame,
        }
    }
}

/// Switch from `prev` to `next`. After returning execution will continue on `next`.
///
/// # Safety
///
/// `prev` and `const` must point to valid processes.
pub unsafe fn switch_process(prev: *const ArchProcess, next: *const ArchProcess) {
    switch_asm(prev, next);

    asm!("iretq");
}

extern "C" {
    #[allow(improper_ctypes)]
    fn switch_asm(prev: *const ArchProcess, next: *const ArchProcess);

    pub fn idle_task() -> !;
}

global_asm!(r#"
    .section .text

    .global switch_asm
    .global idle_task
    .type idle_task, @function
    .type switch_asm, @function

    idle_task:
    0:
        sti
        hlt
        jmp 0b

    switch_asm:
        push rbp
        push rbx
        push r12
        push r13
        push r14
        push r15

        # Swap contexts
        mov [rdi + {off}], rsp
        mov rsp, [rsi + {off}]

        pop r15
        pop r14
        pop r13
        pop r12
        pop rbx
        pop rbp
        jmp finish
"#, off = const offset_of!(ArchProcess, kernel_sp));

/// Finish switching from `prev` to `next` by restoring everything other than the general-purpose
/// registers
#[export_name = "finish"]
extern "C" fn finish(prev: &ArchProcess, next: &ArchProcess) {
    next.addr_space.switch();
}

/// Initial frame for a kernel process
#[repr(C)]
struct KernelFrame {
    pad: [u8; 48],
    frame: InterruptStackFrame,
}

/// /// Writes an initialization frame for a kernel process on `stack`, returning the updated stack
/// pointer
///
/// # Safety
///
/// `stack` must be the top of a valid stack
pub unsafe fn init_kernel(stack: VirtAddr, func: fn() -> !) -> VirtAddr {
    let frame = KernelFrame {
        pad: [0; 48],
        frame: InterruptStackFrame {
            inner: InterruptStackFrameInner {
                rip: func as usize as u64,
                cs: 0x08,
                rflags: 0x200,
                rsp: stack.as_usize() as u64,
                ss: 0x10,
            },
        },
    };
    let frame_stack = stack.as_mut_ptr::<KernelFrame>().sub(1);
    frame_stack.write(frame);
    VirtAddr::new(frame_stack as usize)
}

/// Writes an initialization frame for the idle task on `stack`, returning the updated stack pointer
///
/// # Safety
///
/// `stack` must be the top of a valid stack
pub unsafe fn init_idle(stack: VirtAddr) -> VirtAddr {
    let frame = KernelFrame {
        pad: [0; 48],
        frame: InterruptStackFrame {
            inner: InterruptStackFrameInner {
                rip: idle_task as usize as u64,
                cs: 0x08,
                rflags: 0x200,
                rsp: stack.as_usize() as u64,
                ss: 0x10,
            },
        },
    };
    let stack = stack.as_mut_ptr::<KernelFrame>().sub(1);
    stack.write(frame);
    VirtAddr::new(stack as usize)
}
