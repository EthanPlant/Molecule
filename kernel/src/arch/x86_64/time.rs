//! Timer handling

use super::interrupts::apic::ioapic_setup_irq;
use super::interrupts::{allocate_vector, idt, InterruptStackFrame};
use crate::arch::interrupts::apic::get_local_apic;

const TIMER_IRQ: u8 = 0;

extern "x86-interrupt" fn timer_handler(_stack: InterruptStackFrame) {
    log::debug!("Tick");
    get_local_apic().eoi();
}

/// Initialize the timer.
pub fn init() {
    let vec = allocate_vector();
    idt::set_handler(vec as usize, timer_handler);

    get_local_apic().timer_calibrate(vec);
    ioapic_setup_irq(TIMER_IRQ, vec);
}
