//! # Architecture specific code.
//!
//! This module contains code specific to a given Instruction Set Architecture. Presently the only
//! supported ISA is x86_64.

#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;
