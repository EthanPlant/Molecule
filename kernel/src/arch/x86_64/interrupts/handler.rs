#[repr(C)]
pub struct ScratchRegisters {
    pub r11: usize,
    pub r10: usize,
    pub r9: usize,
    pub r8: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub rdx: usize,
    pub rcx: usize,
    pub rax: usize,
}

impl ScratchRegisters {
    pub fn dump(&self) {
        log::debug!("rax: {:#x}", self.rax);
        log::debug!("rcx: {:#x}", self.rcx);
        log::debug!("rdx: {:#x}", self.rdx);
        log::debug!("rdi: {:#x}", self.rdi);
        log::debug!("rsi: {:#x}", self.rsi);
        log::debug!("r8: {:#x}", self.r8);
        log::debug!("r9: {:#x}", self.r9);
        log::debug!("r10: {:#x}", self.r10);
        log::debug!("r11: {:#x}", self.r11);
    }
}

#[repr(C)]
pub struct PreservedRegisters {
    pub r15: usize,
    pub r14: usize,
    pub r13: usize,
    pub r12: usize,
    pub rbp: usize,
    pub rbx: usize,
}

impl PreservedRegisters {
    pub fn dump(&self) {
        log::debug!("rbx: {:#x}", self.rbx);
        log::debug!("rbp: {:#x}", self.rbp);
        log::debug!("r12: {:#x}", self.r12);
        log::debug!("r13: {:#x}", self.r13);
        log::debug!("r14: {:#x}", self.r14);
        log::debug!("r15: {:#x}", self.r15);
    }
}

#[repr(C)]
pub struct IretRegisters {
    pub rip: usize,
    pub cs: usize,
    pub rflags: usize,
    pub rsp: usize,
    pub ss: usize,
}

impl IretRegisters {
    pub fn dump(&self) {
        log::debug!("rip: {:#x}", self.rip);
        log::debug!("cs: {:#x}", self.cs);
        log::debug!("rflags: {:#x}", self.rflags);
        log::debug!("rsp: {:#x}", self.rsp);
        log::debug!("ss: {:#x}", self.ss);
    }
}

#[repr(C)]
pub struct InterruptStackFrame {
    pub scratch: ScratchRegisters,
    pub preserved: PreservedRegisters,
    pub iret: IretRegisters,
}

impl InterruptStackFrame {
    pub fn dump(&self) {
        self.scratch.dump();
        self.preserved.dump();
        self.iret.dump();
    }
}

pub macro push_scratch() {
    "
            push rcx
            push rdx
            push rdi
            push rsi
            push r8
            push r9
            push r10
            push r11
        "
}

pub macro pop_scratch() {
    "
            pop r11
            pop r10
            pop r9
            pop r8
            pop rsi
            pop rdi
            pop rdx
            pop rcx
            pop rax
        "
}

pub macro push_preserved() {
    "
            push rbx
            push rbp
            push r12
            push r13
            push r14
            push r15
        "
}

pub macro pop_preserved() {
    "
            pop r15
            pop r14
            pop r13
            pop r12
            pop rbp
            pop rbx
        "
}

pub macro interrupt_stack($name:ident, |$stack:ident| $code:block) {

    #[naked]
    pub unsafe extern "C" fn $name() {
        unsafe extern "C" fn inner($stack: &mut InterruptStackFrame) {
            unsafe { $code }
        }

        core::arch::naked_asm!(concat!(
            "cld;",
            "push rax\n",
            $crate::arch::x86_64::interrupts::handler::push_scratch!(),
            $crate::arch::x86_64::interrupts::handler::push_preserved!(),

            "
            mov rdi, rsp
            call {inner}
            ",

            $crate::arch::x86_64::interrupts::handler::pop_preserved!(),
            $crate::arch::x86_64::interrupts::handler::pop_scratch!(),
            "iretq\n",
        ), inner = sym inner,);
    }
}

pub macro interrupt_error($name:ident, |$stack:ident, $error_code:ident| $code:block) {
    #[naked]
    pub unsafe extern "C" fn $name() {
        unsafe extern "C" fn inner($stack: &mut InterruptStackFrame, $error_code: usize) {
            unsafe { $code }
        }

        core::arch::naked_asm!(concat!(
            "cld;",

            $crate::arch::x86_64::interrupts::handler::push_scratch!(),
            $crate::arch::x86_64::interrupts::handler::push_preserved!(),

            "mov rsi, [rsp + {rax_offset}];",
            "mov [rsp + {rax_offset}], rax;",

            "
            mov rdi, rsp
            call {inner}
            ",

            $crate::arch::x86_64::interrupts::handler::pop_preserved!(),
            $crate::arch::x86_64::interrupts::handler::pop_scratch!(),
            "iretq\n",
        ),
        inner = sym inner,
        rax_offset = const(::core::mem::size_of::<$crate::arch::x86_64::interrupts::handler::PreservedRegisters>() + ::core::mem::size_of::<$crate::arch::x86_64::interrupts::handler::ScratchRegisters>() - 8),
        );
    }
}
