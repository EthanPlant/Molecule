use core::{alloc::Layout, arch::{asm, naked_asm}, ptr::Unique};

use alloc::alloc::alloc_zeroed;

use crate::{
    arch::interrupts::handler::{pop_preserved, pop_scratch}, memory::{addr::VirtAddr, frame::FRAME_ALLOCATOR, MapError}
};

use super::{
    gdt::KERNEL_CODE_INDEX, interrupts::handler::{interrupt_stack, InterruptStackFrame},
    paging::address_space::AddressSpace,
};

const SWITCH_STACK_SIZE: usize = 4096 * 4;
const STACK_SIZE: usize = 4096 * 16;

#[derive(Default, Debug)]
#[repr(C)]
struct Context {
    cr3: u64,

    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,

    rbx: u64,
    rbp: u64,

    rip: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct ArchProcess {
    context: Unique<Context>,

    addr_space: AddressSpace,
    context_switch_rsp: VirtAddr,
}

impl ArchProcess {
    pub fn new_kernel(entry_point: VirtAddr, enable_int: bool) -> Self {
        let switch_stack = Self::alloc_stack().as_mut_ptr::<u8>();

        let process_stack = unsafe {
            let layout = Layout::from_size_align_unchecked(STACK_SIZE, 0x1000);
            alloc_zeroed(layout).add(layout.size())
        };

        let addr_space = AddressSpace::this();

        let mut stack_ptr = switch_stack as usize;

        let kframe = unsafe {
            stack_ptr -=
                core::mem::size_of::<InterruptStackFrame>();
            &mut *(stack_ptr as *mut InterruptStackFrame)
        };

        kframe.iret.ss = 0x10;
        kframe.iret.cs = 0x08;
        kframe.iret.rip = usize::from(entry_point);
        kframe.iret.rsp = process_stack as usize;
        kframe.iret.rflags = if enable_int { 0x200 } else { 0x00 };

        let context = unsafe {
            stack_ptr -= core::mem::size_of::<Context>();
            &mut *(stack_ptr as *mut Context)
        };

        *context = Context::default();
        context.rip = iretq_init as u64;
        context.cr3 = addr_space.cr3();

        Self {
            context: unsafe { Unique::new_unchecked(context) },
            addr_space,
            context_switch_rsp: VirtAddr::new(switch_stack as usize),
        }
    }

    fn alloc_stack() -> VirtAddr {
        let frame = FRAME_ALLOCATOR
            .alloc_zeroed(SWITCH_STACK_SIZE)
            .expect("Failed to allocate stack frame");
        frame.as_hddm_virt() + STACK_SIZE
    }
}

pub fn arch_switch_process(from: &mut ArchProcess, to: &ArchProcess) {
    unsafe {
        process_spinup(&mut from.context, to.context.as_ref());
    }
}

pub fn idle_process() {
    loop {
        unsafe { asm!("hlt"); }
    }
}

#[naked]
unsafe extern "C" fn process_spinup(prev: &mut Unique<Context>, next: &Context) {
    naked_asm!(
        "push rbp",
        "push rbx",
        "push r12",
        "push r13",
        "push r14",
        "push r15",
        "mov rax, cr3",
        "push rax",
        "mov [rdi], rsp",
        "mov rsp, rsi",
        "pop rax",
        "mov cr3, rax",
        "pop r15",
        "pop r14",
        "pop r13",
        "pop r12",
        "pop rbx",
        "pop rbp",
        "ret",
    )
}

#[naked]
unsafe extern "C" fn iretq_init() {
    naked_asm!(
        "cli",
        pop_preserved!(),
        pop_scratch!(),
        "iretq",
    )
}
