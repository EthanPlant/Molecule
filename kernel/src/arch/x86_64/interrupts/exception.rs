//! Exception handlers for ``x86_64`` CPU instructions.

bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy)]
    struct PageFaultErrorCode: usize {
        const PROTECTION_VIOLATION = 1 << 0;
        const CAUSED_BY_WRITE = 1 << 1;
        const USER_MODE = 1 << 2;
        const MALFORMED_TABLE = 1 << 3;
        const INSTRUCTION_FETCH = 1 << 4;
        const PROTECTION_KEY = 1 << 5;
        const SHADOW_STACK = 1 << 6;
        const SGX = 1 << 15;
        const RMP = 1 << 31;
    }
}

use core::arch::asm;

use super::{
    handler::{interrupt_error, interrupt_stack},
    idt::IDT,
};

interrupt_stack!(divide_by_zero, |stack| {
    stack.dump();
    panic!("Divide by zero exception")
});

interrupt_stack!(debug, |stack| {
    stack.dump();
    panic!("Debug exception")
});

interrupt_stack!(non_maskable_interrupt, |stack| {
    stack.dump();
    panic!("Non-maskable interrupt exception")
});

interrupt_stack!(breakpoint, |stack| {
    log::debug!("Breakpoint hit!");
    stack.dump();
});

interrupt_stack!(overflow, |stack| {
    stack.dump();
    panic!("Overflow exception")
});

interrupt_stack!(bound_range_exceeded, |stack| {
    stack.dump();
    panic!("Bound range exceeded exception")
});

interrupt_stack!(invalid_opcode, |stack| {
    stack.dump();
    panic!("Invalid opcode exception")
});

interrupt_stack!(device_not_available, |stack| {
    stack.dump();
    panic!("Device not available exception")
});

interrupt_error!(double_fault, |stack, error_code| {
    stack.dump();
    panic!("Double fault exception with error code: {}", error_code)
});

interrupt_error!(invalid_tss, |stack, error_code| {
    stack.dump();
    panic!("Invalid TSS exception with error code: {}", error_code)
});

interrupt_error!(segment_not_present, |stack, error_code| {
    stack.dump();
    panic!(
        "Segment not present exception with error code: {}",
        error_code
    )
});

interrupt_error!(stack_segment_fault, |stack, error_code| {
    stack.dump();
    panic!(
        "Stack segment fault exception with error code: {}",
        error_code
    )
});

interrupt_error!(general_protection_fault, |stack, error_code| {
    stack.dump();
    panic!(
        "General protection fault exception with error code: {}",
        error_code
    )
});

interrupt_error!(page_fault, |stack, error_code| {
    stack.dump();
    let addr = unsafe {
        let cr2: usize;
        asm!("mov {}, cr2", out(reg) cr2);
        cr2
    };
    panic!(
        "Page fault exception with error code: {:?} at address {:x}",
        PageFaultErrorCode::from_bits_truncate(error_code),
        addr
    )
});

interrupt_stack!(x87_floating_point, |stack| {
    stack.dump();
    panic!("x87 floating point exception")
});

interrupt_error!(alignment_check, |stack, error_code| {
    stack.dump();
    panic!("Alignment check exception with error code: {}", error_code)
});

interrupt_stack!(machine_check, |stack| {
    stack.dump();
    panic!("Machine check exception")
});

interrupt_stack!(simd_floating_point, |stack| {
    stack.dump();
    panic!("SIMD floating point exception")
});

interrupt_stack!(virtualization, |stack| {
    stack.dump();
    panic!("Virtualization exception")
});

interrupt_error!(control_protection, |stack, error_code| {
    stack.dump();
    panic!(
        "Control protection exception with error code: {}",
        error_code
    )
});

interrupt_stack!(hypervisor_injection, |stack| {
    stack.dump();
    panic!("Hypervisor injection exception")
});

interrupt_error!(vmm_communication, |stack, error_code| {
    stack.dump();
    panic!(
        "VMM communication exception with error code: {}",
        error_code
    )
});

interrupt_error!(security_exception, |stack, error_code| {
    stack.dump();
    panic!("Security exception with error code: {}", error_code)
});

/// Registers handlers for CPU exceptions into the IDT.
pub fn register_exceptions() {
    let mut lock = IDT.lock();
    // Safety: All of these are valid handler functions generated by the interrupt_stack and interrupt_error macros.
    unsafe {
        lock.entries[0].set_func(divide_by_zero);
        lock.entries[1].set_func(debug);
        lock.entries[2].set_func(non_maskable_interrupt);
        lock.entries[3].set_func(breakpoint);
        lock.entries[4].set_func(overflow);
        lock.entries[5].set_func(bound_range_exceeded);
        lock.entries[6].set_func(invalid_opcode);
        lock.entries[7].set_func(device_not_available);
        lock.entries[8].set_func(double_fault);
        lock.entries[10].set_func(invalid_tss);
        lock.entries[11].set_func(segment_not_present);
        lock.entries[12].set_func(stack_segment_fault);
        lock.entries[13].set_func(general_protection_fault);
        lock.entries[14].set_func(page_fault);
        lock.entries[16].set_func(x87_floating_point);
        lock.entries[17].set_func(alignment_check);
        lock.entries[18].set_func(machine_check);
        lock.entries[19].set_func(simd_floating_point);
        lock.entries[20].set_func(virtualization);
        lock.entries[21].set_func(control_protection);
        lock.entries[28].set_func(hypervisor_injection);
        lock.entries[29].set_func(vmm_communication);
        lock.entries[30].set_func(security_exception);
    }
}
