//! ``x86_64`` interrupt handling.

use core::arch::asm;

use apic::{get_local_apic, ioapic_setup_irq, LocalApic, LOCAL_APIC};
use handler::{interrupt_stack, HandlerFunc};
use idt::{IdtEntry, IDT};
use spin::Mutex;

use crate::{
    arch::io::{inb, outb},
    drivers::framebuffer::console::print,
    TICKS,
};

use super::io;

pub mod apic;
pub mod exception;
pub mod handler;
pub mod idt;

pub fn register_handler(vector: u8, handler: HandlerFunc) {
    let mut handlers = IDT.lock();

    assert!(
        handlers.entries[vector as usize] == IdtEntry::EMPTY,
        "Handler already registered! {:x?}",
        handlers.entries[vector as usize]
    );

    unsafe { handlers.entries[vector as usize].set_func(handler) };
}

pub fn allocate_vector() -> u8 {
    static IDT_FREE_VECTOR: Mutex<u8> = Mutex::new(32);

    let mut vector = IDT_FREE_VECTOR.lock();
    let copy = *vector;

    assert!((copy != 0xF0), "Vector allocation exhausted!");

    *vector += 1;
    copy
}

pub fn disable_pic() {
    unsafe {
        outb(0x21, 0xFF);
        outb(0x80, 0);
        outb(0xA1, 0xff);
        outb(0x80, 0x00);
    }

    log::debug!("PIC Disabled");
}

pub fn enable_interrupts() {
    unsafe {
        asm!("sti");
    }
}
