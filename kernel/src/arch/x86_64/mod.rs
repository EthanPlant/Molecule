//! # x86_64
//!
//! This module contains all architecture specific code for the x86_64 (AMD64) ISA.

use interrupts::apic::get_local_apic;

mod gdt;
mod init;
pub mod interrupts;
pub mod io;
mod memory;
pub mod process;
mod time;

pub fn get_cpu_id() -> u32 {
    get_local_apic().get_lapic_id() >> 24
}

/// Represents a privilege level.
enum PrivilegeLevel {
    Kernel = 0,
    User = 3,
}

impl From<u8> for PrivilegeLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => PrivilegeLevel::Kernel,
            3 => PrivilegeLevel::User,
            _ => unreachable!("Invalid privilege level"),
        }
    }
}
