//! Root System Descriptor Pointer (RSDP)

use super::rsdt::RsdtAddr;
use crate::memory::addr::{PhysAddr, VirtAddr};

/// In memory representation of a version 1 RSDP.
#[repr(C, packed)]
struct RsdpV1 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_addr: u32,
}

/// In memory representation of a version 1 RSDP. A version 2 RSDP is identical to [RsdpV1] with
/// some additional fields.
#[repr(C, packed)]
struct RsdpV2 {
    _signature: [u8; 8],
    _checksum: u8,
    _oem_id: [u8; 6],
    _revision: u8,
    _rsdt_addr: u32,

    _length: u32,
    xsdt_addr: u64,
    _extended_checksum: u8,
    _reserved: [u8; 3],
}

/// Find the address of the RSDT or XSDT from the RSDP pointed to by `resp`. Returns
/// [RsdtAddr::Rsdt] for a V1 RSDP and [RsdtAddr::Xsdt] for a V2 RSDP.
pub(super) fn find_rsdt_addr(resp: &limine::response::RsdpResponse) -> RsdtAddr {
    let addr = VirtAddr::new(resp.address() as usize);
    log::debug!("ACPI: RSDP found at at {:?}", resp.address());
    // Safety: The response provided to us by Limine contains the address of the RSDP. As a V2 RSDP
    // has the same initial 20 bytes as a V1 RSDP. This is valid for either a V1 or V2 RSDP.
    let rsdp = unsafe { &*addr.as_ptr::<RsdpV1>() };
    let is_v2 = rsdp.revision >= 2;

    if is_v2 {
        // Safety: Since we know the RSDP is a V2 RSDP, dereferencing the remaining bytes is valid.
        let rsdp = unsafe { &*addr.as_ptr::<RsdpV2>() };
        RsdtAddr::Xsdt(PhysAddr::new(rsdp.xsdt_addr as usize).as_hddm_virt())
    } else {
        RsdtAddr::Rsdt(PhysAddr::new(rsdp.rsdt_addr as usize).as_hddm_virt())
    }
}
