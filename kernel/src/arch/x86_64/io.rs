use core::arch::asm;

/// Wrapper function around the outb asm instruction
///
/// # Safety
///
/// The caller must ensure that the port is valid as
/// attempting to write to a non-existent port
/// can lead to undefined behavior.
pub unsafe fn outb(port: u16, value: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(preserves_flags, nomem, nostack)
    );
}

/// Wrapper function around the inb asm instruction
///
/// # Safety
///
/// The caller must ensure that the port is valid as
/// attempting to read from a non-existent port can lead to undefined behavior.
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!(
        "in al, dx",
        out("al") value,
        in("dx") port,
        options(preserves_flags, nomem, nostack)
    );
    value
}

/// Wrapper function around the outw asm instruction
///
/// # Safety
///
/// The caller must ensure that the port is valid as
/// attempting to write to a non-existent port
/// can lead to undefined behavior.
#[allow(dead_code)]
pub unsafe fn outw(port: u16, value: u16) {
    asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
        options(preserves_flags, nomem, nostack)
    );
}

/// Wrapper function around the inw asm instruction
///
/// # Safety
///
/// The caller must ensure that the port is valid as
/// attempting to read from a non-existent port can lead to undefined behavior.
#[allow(dead_code)]
pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    asm!(
        "in ax, dx",
        out("ax") value,
        in("dx") port,
        options(preserves_flags, nomem, nostack)
    );
    value
}

/// Wrapper function around the outl asm instruction
///
/// # Safety
///
/// The caller must ensure that the port is valid as
/// attempting to write to a non-existent port can lead to undefined behavior.
#[allow(dead_code)]
pub unsafe fn outl(port: u16, value: u32) {
    asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") value,
        options(preserves_flags, nomem, nostack)
    );
}

/// Wrapper function around the inl asm instruction
///
/// # Safety
///
/// The caller must ensure that the port is valid as
/// attempting to read from a non-existent port can lead to undefined behavior.
#[allow(dead_code)]
pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    asm!(
        "in eax, dx",
        out("eax") value,
        in("dx") port,
        options(preserves_flags, nomem, nostack)
    );
    value
}

pub unsafe fn rdmsr(msr: u32) -> u64 {
    let (high, low): (u32, u32);

    asm!("rdmsr", out("eax") low, out("edx") high, in("ecx") msr, options(nomem));

    ((high as u64) << 32) | (low as u64)
}

pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;

    asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nomem));
}
