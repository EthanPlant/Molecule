//! Exception handlers for ``x86_64`` CPU instructions.

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
    panic!("Page fault exception with error code: {}", error_code)
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
    unsafe {
        IDT[0].set_func(divide_by_zero);
        IDT[1].set_func(debug);
        IDT[2].set_func(non_maskable_interrupt);
        IDT[3].set_func(breakpoint);
        IDT[4].set_func(overflow);
        IDT[5].set_func(bound_range_exceeded);
        IDT[6].set_func(invalid_opcode);
        IDT[7].set_func(device_not_available);
        IDT[8].set_func(double_fault);
        IDT[10].set_func(invalid_tss);
        IDT[11].set_func(segment_not_present);
        IDT[12].set_func(stack_segment_fault);
        IDT[13].set_func(general_protection_fault);
        IDT[14].set_func(page_fault);
        IDT[16].set_func(x87_floating_point);
        IDT[17].set_func(alignment_check);
        IDT[18].set_func(machine_check);
        IDT[19].set_func(simd_floating_point);
        IDT[20].set_func(virtualization);
        IDT[21].set_func(control_protection);
        IDT[28].set_func(hypervisor_injection);
        IDT[29].set_func(vmm_communication);
        IDT[30].set_func(security_exception);
    }
}
