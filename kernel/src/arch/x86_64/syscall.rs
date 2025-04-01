use core::arch::asm;

use super::interrupts::{idt, InterruptStackFrame};

pub extern "x86-interrupt" fn syscall_handler(_stack: InterruptStackFrame) {
    let syscall_number: u64;
    unsafe {
        asm!(
            "mov {syscall_number}, rax",
            syscall_number = out(reg) syscall_number,
        );
    }
    log::trace!("Recieved syscall number: {syscall_number}");
}

pub fn init() {
    idt::set_handler(0x80, syscall_handler);
    log::debug!("Syscall handler initialized");
}
