//! Exception handling

use super::{idt::IDT, InterruptStackFrame};

extern "x86-interrupt" fn unhandled_exception(stack: InterruptStackFrame) {
    log::debug!("{:#x?}", stack);
    panic!("Unhandled exception occured");
}

extern "x86-interrupt" fn double_fault(stack: InterruptStackFrame, _err_code: u64) {
    log::debug!("{:#x?}", stack);
    panic!("Double fault (#DF)");
}

extern "x86-interrupt" fn invalid_tss(stack: InterruptStackFrame, err_code: u64) {
    log::debug!("{:#x?}", stack);
    panic!("Invalid TSS (#TS) with index {:x}", err_code);
}

extern "x86-interrupt" fn segment_not_present(stack: InterruptStackFrame, err_code: u64) {
    log::debug!("{:#x?}", stack);
    panic!("Attempted to load not present segment with index {:x} (#NP)", err_code);
}

extern "x86-interrupt" fn stack_segment(stack: InterruptStackFrame, err_code: u64) {
    log::debug!("{:#x?}", stack);
    panic!("Stack segment fault (#SS) with error code {:x}", err_code);
}

extern "x86-interrupt" fn gpf(stack: InterruptStackFrame, err_code: u64) {
    log::debug!("{:#x?}", stack);
    panic!("General Protection Fault (#GP) with error code {:x}", err_code);
}

extern "x86-interrupt" fn page_fault(stack: InterruptStackFrame, err_code: u64) {
    log::debug!("{:#x?}", stack);
    panic!("Page Fault (#PF) with error code {:x}", err_code);
}

extern "x86-interrupt" fn alignment_check(stack: InterruptStackFrame, err_code: u64) {
    log::debug!("{:#x?}", stack);
    panic!("Alignment Check (#AC) with error code {:x}", err_code);
}

extern "x86-interrupt" fn control_protection(stack: InterruptStackFrame, err_code: u64) {
    log::debug!("{:#x?}", stack);
    panic!("Control Protection Exception (#CP) with error code {:x}", err_code);
}

extern "x86-interrupt" fn vmm_comm(stack: InterruptStackFrame, err_code: u64) {
    log::debug!("{:#x?}", stack);
    panic!("VMM Communication Exception (#VC) with error code {:x}", err_code);
}

/// Register CPU exceptions to their handlers
pub fn register_exceptions() {
    let mut lock = IDT.write();
        lock.set_handler(0, unhandled_exception);
        lock.set_handler(1, unhandled_exception);
        lock.set_handler(2, unhandled_exception);
        lock.set_handler(3, unhandled_exception);
        lock.set_handler(4, unhandled_exception);
        lock.set_handler(5, unhandled_exception);
        lock.set_handler(6, unhandled_exception);
        lock.set_handler(7, unhandled_exception);
        lock.set_handler_with_error_code(8, double_fault);
        lock.set_handler(9, unhandled_exception);
        lock.set_handler_with_error_code(10, invalid_tss);
        lock.set_handler_with_error_code(11, segment_not_present);
        lock.set_handler_with_error_code(12, stack_segment);
        lock.set_handler_with_error_code(13, gpf);
        lock.set_handler_with_error_code(14, page_fault);
        lock.set_handler(15, unhandled_exception);
        lock.set_handler(16, unhandled_exception);
        lock.set_handler_with_error_code(17, alignment_check);
        lock.set_handler(18, unhandled_exception);
        lock.set_handler(19, unhandled_exception);
        lock.set_handler(20, unhandled_exception);
        lock.set_handler_with_error_code(21, control_protection);
        lock.set_handler(22, unhandled_exception);
        lock.set_handler(23, unhandled_exception);
        lock.set_handler(24, unhandled_exception);
        lock.set_handler(25, unhandled_exception);
        lock.set_handler(26, unhandled_exception);
        lock.set_handler(27, unhandled_exception);
        lock.set_handler(28, unhandled_exception);
        lock.set_handler_with_error_code(29, vmm_comm);
        lock.set_handler(30, unhandled_exception);
        lock.set_handler(31, unhandled_exception);
}