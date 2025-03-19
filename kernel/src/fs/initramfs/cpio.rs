//! CPIO parsing

use core::mem;

use limine::file;

const CPIO_MAGIC: u16 = 0o070707;

/// CPIO Archive parser.
pub struct CpioParser<'a> {
    data: &'a [u8],
    off: usize,
}

impl<'a> CpioParser<'a> {
    /// Create a new instance of the parser for a given data slice.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, off: 0 }
    }
}

impl<'a> Iterator for CpioParser<'a> {
    type Item = CpioEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.off >= self.data.len() {
            return None;
        }

        if mem::size_of::<CpioHeader>() > self.data[self.off..].len() {
            return None;
        }
        // Safety: Slice is large enough to contain a CPIO header
        let header = unsafe { &*self.data[self.off..].as_ptr().cast::<CpioHeader>() };
        if header.magic != 0o070707 {
            log::warn!("Invalid header");
            return None;
        }
        let mut namesize = header.namesize as usize;
        if namesize % 2 != 0 {
            namesize += 1;
        }
        let mut filesize = header.filesize.rotate_left(16) as usize;
        if filesize % 2 != 0 {
            filesize += 1;
        }
        let size = mem::size_of::<CpioHeader>() + namesize + filesize;
        let overflow = self
            .off
            .checked_add(size)
            .map(|end| end > self.data.len())
            .unwrap_or(true);
        if overflow {
            log::warn!("Overflow");
            return None;
        }
        let entry = CpioEntry {
            data: &self.data[self.off..(self.off + size)],
        };
        self.off += size;
        if entry.get_filename() == b"TRAILER!!!" {
            return None;
        }
        Some(entry)
    }
}

/// A CPIO entry, consisting of a CPIO header, the filename, and the content of the file
pub struct CpioEntry<'a> {
    data: &'a [u8],
}

impl<'a> CpioEntry<'a> {
    pub fn get_header(&self) -> &'a CpioHeader {
        // Safety: The header is always at the beginning of a CPIO entry
        unsafe { &*self.data.as_ptr().cast::<CpioHeader>() }
    }

    pub fn get_filename(&self) -> &'a [u8] {
        let header = self.get_header();
        let start = mem::size_of::<CpioHeader>();
        let mut end = start + header.namesize as usize;
        if end - start > 0 && self.data[end - 1] == b'\0' {
            end -= 1;
        }
        &self.data[start..end]
    }

    pub fn get_content(&self) -> &'a [u8] {
        let header = self.get_header();
        let mut start = size_of::<CpioHeader>() + header.namesize as usize;
        if start % 2 != 0 {
            start += 1;
        }
        let filesize = header.filesize.rotate_left(16);
        &self.data[start..(start + filesize as usize)]
    }
}

/// A CPIO entry header
#[repr(C, packed)]
pub struct CpioHeader {
    magic: u16,
    dev: u16,
    ino: u16,
    pub mode: u16,
    pub uid: u16,
    pub gid: u16,
    link: u16,
    pub rdev: u16,
    mtime: u32,
    namesize: u16,
    filesize: u32,
}
