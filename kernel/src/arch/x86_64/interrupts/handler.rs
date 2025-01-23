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

/// Represents the stack frame pushed by the CPU during an interrupt.
///
/// This contains the state of all registers at the time of the interrupt, so that we can preserve them and restore them when the interrupt handler is done.
#[repr(C)]
pub struct InterruptStackFrame {
    pub scratch: ScratchRegisters,
    pub preserved: PreservedRegisters,
    pub iret: IretRegisters,
}

impl InterruptStackFrame {
    /// Dump the contents of the CPU registers to the logger.
    pub fn dump(&self) {
        self.scratch.dump();
        self.preserved.dump();
        self.iret.dump();
    }
}

/// Push all of the [`ScratchRegisters`] to the stack.
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

/// Pop all of the [`ScratchRegisters`] from the stack.
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

/// Push all of the [`PreservedRegisters`] to the stack.
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

/// Pop all of the [`PreservedRegisters`] from the stack.
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

/// Generate an interrupt handler with access to the stack frame.
///
/// This macro accepts a name, an identifier for the stack frame, and a block of code to execute.
/// The macro produces a naked function which calls the provided block of code with the stack frame as an argument.
/// This enables the inner line of code to access the stack frame and the CPU registers at the time of the interrupt.
///
/// # Example
/// ```
/// interrupt_stack!(divide_by_zero, |stack| {
///    stack.dump();
///    panic!("Divide by zero exception");
/// });
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

/// Generate an interrupt handler with access to the stack frame and an error code.
///
/// This macro operates nearly identical to [`interrupt_stack`], but also provides an error code to the inner block of code.
/// This is mostly used for certain exceptions which provide an additional error code.
///
/// # Example
/// ```
/// interrupt_error!(page_fault, |stack, error_code| {
///   stack.dump();
///   panic!("Page fault exception with error code: {}", error_code);
/// });
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
