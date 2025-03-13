//! SIMD handling

use core::arch::asm;

use raw_cpuid::{CpuId, FeatureInfo};

/// Enable monitoring of the coprocessor
const MONITOR_COPROCESSOR: u64 = 1 << 1;
/// Force ann x87 and MMX instructions to cause a #NE exception
const EMULATE_COPROCESSOR: u64 = 1 << 2;

/// Enables the use of legacy SSE instructions.
const OSFXSR: u64 = 1 << 9;
/// Enables the SIMD floating-point exception (#XF) for handling unmasked 256-bit and 128-bit media
/// floating point errors.
const OSXMMEXCPT_ENABLE: u64 = 1 << 10;
/// Enables software running in 64-bit mode at any privilege level to read and write to the FS.base
/// and GS.base hidden segment registers
const FSGSBASE: u64 = 1 << 16;
/// Enables extended processor state management instructions, including XGETBV and XSAVE
const OSXSAVE: u64 = 1 << 18;

/// Enables using the x87 FPU state with XSAVE/XRSTORE
const X87: u64 = 1;
/// Enables using MXCSR and the XMM registers with XSAVE/XRSTORE
const SSE: u64 = 1 << 1;
/// Enables AVX instructions and using the upper halves of the AVX registers with XSAVE/XRSTORE
const AVX: u64 = 1 << 2;

/// Initialize SSE and AVX
pub fn init() {
    let features = CpuId::new().get_feature_info().unwrap();
    assert!(features.has_sse(), "Init: SSE not supported");
    // Safety: SSE is supported, so enabling it is sound
    unsafe {
        let mut cr0: u64;
        asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack, preserves_flags));
        cr0 &= !EMULATE_COPROCESSOR;
        cr0 |= MONITOR_COPROCESSOR;
        asm!("mov cr0, {}", in(reg) cr0, options(nostack, preserves_flags));

        let mut cr4: u64;
        asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));
        cr4 |= OSFXSR;
        cr4 |= OSXMMEXCPT_ENABLE;

        if CpuId::new()
            .get_extended_feature_info()
            .unwrap()
            .has_fsgsbase()
        {
            cr4 |= FSGSBASE;
        }

        asm!("mov {}, cr4", in(reg) cr4, options(nostack, preserves_flags));
    }
    init_xsave(&features);
}

/// Initialize XSAVE for saving and loading the SSE context.
fn init_xsave(features: &FeatureInfo) {
    assert!(features.has_xsave(), "Init: XSAVE not supported");
    // Safety: CPU supports XSAVE, so we can safely initialize it
    unsafe {
        let mut cr4: u64;
        asm!("mov {}, cr4", out(reg) cr4, options(nostack, preserves_flags));
        cr4 |= OSXSAVE;
        asm!("mov cr4, {}", in(reg) cr4, options(nostack, preserves_flags));

        let (mut low, mut high): (u32, u32);
        asm!("xgetbv", in("ecx") 0, out("rax") low, out("rdx") high, options(nomem, nostack, preserves_flags));
        let mut xcr0 = (high as u64) << 32 | low as u64;
        xcr0 |= X87 | SSE | AVX;
        low = xcr0 as u32;
        high = (xcr0 >> 32) as u32;
        asm!("xsetbv", in("ecx") 0, in("rax") low, in("rdx") high, options(nomem, nostack, preserves_flags));
    }
}
