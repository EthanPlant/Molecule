use core::{
    fmt::{self, Debug},
    str,
};

use crate::memory::addr::PhysAddr;

const RSDP_SIG: [u8; 8] = *b"RSD PTR ";

const RSDP_V1_LEN: usize = 20;

#[derive(Debug)]
enum RsdpError {
    InvalidSignature,
    InvalidChecksum,
    InvalidOemId,
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oem_id: [u8; 6],
    revision: u8,
    rsdt_addr: u32,
    length: u32,
    xsdt_addr: u64,
    extended_checksum: u8,
    reserved: [u8; 3],
}

impl Rsdp {
    pub fn new(resp: &limine::response::RsdpResponse) -> Self {
        let rsdp = unsafe { &*resp.address().cast::<Rsdp>() };
        let valid = rsdp.validate();
        assert!(valid.is_ok(), "Failed to validate RSDP {valid:?}");

        *rsdp
    }

    pub fn revision(&self) -> u8 {
        self.revision
    }

    pub fn rsdt_addr(&self) -> PhysAddr {
        PhysAddr::new(self.rsdt_addr as usize)
    }

    pub fn xsdt_addr(&self) -> Option<PhysAddr> {
        if self.revision == 0 {
            None
        } else {
            Some(PhysAddr::new(self.xsdt_addr as usize))
        }
    }

    fn validate(&self) -> Result<(), RsdpError> {
        if self.signature != RSDP_SIG {
            return Err(RsdpError::InvalidSignature);
        }

        if str::from_utf8(&self.oem_id).is_err() {
            return Err(RsdpError::InvalidOemId);
        }

        let length = if self.revision == 0 {
            RSDP_V1_LEN
        } else {
            self.length as usize
        };

        let bytes = unsafe {
            core::slice::from_raw_parts(core::ptr::from_ref::<Rsdp>(self).cast::<u8>(), length)
        };
        let sum = bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte));
        if sum != 0 {
            return Err(RsdpError::InvalidChecksum);
        }

        Ok(())
    }
}

impl fmt::Debug for Rsdp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut f = f.debug_struct("Rsdp");
        let rsdt_addr = self.rsdt_addr;
        let xsdt_addr = self.xsdt_addr;
        let length = self.length;
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
        f.finish_non_exhaustive()
    }
}
