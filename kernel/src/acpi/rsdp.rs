use core::{mem, str};

use alloc::fmt;

use crate::memory::addr::{PhysAddr, VirtAddr};

use super::rsdt::RsdtAddr;

const RSDP_SIGNATURE: [u8; 8] = *b"RSD PTR ";

pub(crate) trait Rsdp {
    fn validate_rsdp(&self) -> bool;
}

#[repr(C, packed)]
pub struct RsdpV1 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_addr: u32,
}

impl fmt::Debug for RsdpV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Rsdp");
        let rsdt_addr = self.rsdt_addr;
        f.field("signature", unsafe {
            &str::from_utf8_unchecked(&self.signature)
        });
        f.field("checksum", &self.checksum);
        f.field("oem_id", unsafe { &str::from_utf8_unchecked(&self.oem_id) });
        f.field("revision", &self.revision);
        f.field("rsdt_addr", &format_args!("{:x}", &rsdt_addr));
        f.finish()
    }
}

impl Rsdp for RsdpV1 {
    fn validate_rsdp(&self) -> bool {
        if self.signature == RSDP_SIGNATURE {
            let ptr = self as *const _ as *const u8;
            return unsafe { validate_checksumn(ptr, mem::size_of::<Self>()) };
        }

        false
    }
}

#[repr(C, packed)]
struct RsdpV2 {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_addr: u32,

    length: u32,
    xsdt_addr: u32,
    extended_checksum: u8,
    reserved: [u8; 3],
}

impl fmt::Debug for RsdpV2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Rsdp");
        let rsdt_addr = self.rsdt_addr;
        let length = self.length;
        let xsdt_addr = self.xsdt_addr;
        f.field("signature", unsafe {
            &str::from_utf8_unchecked(&self.signature)
        });
        f.field("checksum", &self.checksum);
        f.field("oem_id", unsafe { &str::from_utf8_unchecked(&self.oem_id) });
        f.field("revision", &self.revision);
        f.field("rsdt_addr", &format_args!("{:x}", &rsdt_addr));
        f.field("length", &length);
        f.field("xsdt_addr", &format_args!("{:x}", &xsdt_addr));
        f.field("extended_checksum", &self.extended_checksum);

        f.finish()
    }
}

impl Rsdp for RsdpV2 {
    fn validate_rsdp(&self) -> bool {
        if self.signature == RSDP_SIGNATURE {
            let ptr = self as *const _ as *const u8;
            return unsafe { validate_checksumn(ptr, self.length as usize) };
        }

        false
    }
}

pub fn find_rsdt_addr(rsdp_addr: VirtAddr) -> RsdtAddr {
    let v2 = unsafe { &*rsdp_addr.as_ptr::<RsdpV2>() };
    let is_v2 = v2.revision >= 2 && v2.xsdt_addr != 0;

    if is_v2 {
        assert!(v2.validate_rsdp(), "Failed to validate RSDP v2 checksum");

        let xsdt_address = PhysAddr::new(v2.xsdt_addr as usize).as_hddm_virt();
        RsdtAddr::Xsdt(xsdt_address)
    } else {
        let v1 = unsafe { &*rsdp_addr.as_ptr::<RsdpV1>() };
        assert!(v2.validate_rsdp(), "Failed to validate RSDP checksum");

        let rsdt_address = PhysAddr::new(v1.rsdt_addr as usize).as_hddm_virt();
        RsdtAddr::Rsdt(rsdt_address)
    }
}

unsafe fn validate_checksumn(ptr: *const u8, size: usize) -> bool {
    let mut sum: u8 = 0;
    for i in 0..size {
        sum = sum.wrapping_add(*(ptr.add(i)));
    }

    sum == 0
}

// use core::{
//     fmt::{self, Debug},
//     str,
// };

// use crate::memory::addr::PhysAddr;

// const RSDP_SIG: [u8; 8] = *b"RSD PTR ";

// const RSDP_V1_LEN: usize = 20;

// #[derive(Debug)]
// enum RsdpError {
//     InvalidSignature,
//     InvalidChecksum,
//     InvalidOemId,
// }

// #[repr(C, packed)]
// #[derive(Copy, Clone)]
// pub struct Rsdp {
//     signature: [u8; 8],
//     checksum: u8,
//     oem_id: [u8; 6],
//     revision: u8,
//     rsdt_addr: u32,
//     length: u32,
//     xsdt_addr: u64,
//     extended_checksum: u8,
//     reserved: [u8; 3],
// }

// impl Rsdp {
//     pub fn new(resp: &limine::response::RsdpResponse) -> Self {
//         let rsdp = unsafe { &*resp.address().cast::<Rsdp>() };
//         let valid = rsdp.validate();
//         assert!(valid.is_ok(), "Failed to validate RSDP {valid:?}");

//         *rsdp
//     }

//     pub fn revision(&self) -> u8 {
//         self.revision
//     }

//     pub fn rsdt_addr(&self) -> PhysAddr {
//         PhysAddr::new(self.rsdt_addr as usize)
//     }

//     pub fn xsdt_addr(&self) -> Option<PhysAddr> {
//         if self.revision == 0 {
//             None
//         } else {
//             Some(PhysAddr::new(self.xsdt_addr as usize))
//         }
//     }

//     fn validate(&self) -> Result<(), RsdpError> {
//         if self.signature != RSDP_SIG {
//             return Err(RsdpError::InvalidSignature);
//         }

//         if str::from_utf8(&self.oem_id).is_err() {
//             return Err(RsdpError::InvalidOemId);
//         }

//         let length = if self.revision == 0 {
//             RSDP_V1_LEN
//         } else {
//             self.length as usize
//         };

//         let bytes = unsafe {
//             core::slice::from_raw_parts(core::ptr::from_ref::<Rsdp>(self).cast::<u8>(), length)
//         };
//         let sum = bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
//         if sum != 0 {
//             return Err(RsdpError::InvalidChecksum);
//         }

//         Ok(())
//     }
// }

// impl fmt::Debug for Rsdp {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let mut f = f.debug_struct("Rsdp");
//         let rsdt_addr = self.rsdt_addr;
//         let xsdt_addr = self.xsdt_addr;
//         let length = self.length;
//         f.field("signature", unsafe {
//             &str::from_utf8_unchecked(&self.signature)
//         });
//         f.field("checksum", &self.checksum);
//         f.field("oem_id", unsafe { &str::from_utf8_unchecked(&self.oem_id) });
//         f.field("revision", &self.revision);
//         f.field("rsdt_addr", &format_args!("{:x}", &rsdt_addr));
//         f.field("length", &length);
//         f.field("xsdt_addr", &format_args!("{:x}", &xsdt_addr));
//         f.field("extended_checksum", &self.extended_checksum);
//         f.finish_non_exhaustive()
//     }
// }
