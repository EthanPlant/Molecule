//! # x86_64
//!
//! This module contains all architecture specific code for the x86_64 (AMD64) ISA.

mod gdt;
mod init;
pub mod interrupts;
pub mod io;
mod paging;

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
