//! Abstractions around a linear framebuffer.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicPtr, Ordering};

use color::Color;
use spin::Once;

use crate::fs::devfs::{Device, DeviceId, DeviceType};
use crate::psf::PsfFont;
use crate::sync::{Mutex, MutexGuard};

mod color;
pub mod console;

/// The global framebuffer.
static FRAMEBUFFER: Once<Mutex<FramebufferInfo>> = Once::new();

/// In-memory representation of a linear framebuffer
pub struct FramebufferInfo {
    addr: AtomicPtr<u32>,
    width: usize,
    height: usize,
    pitch: usize,
}

impl FramebufferInfo {
    /// Create a new [FramebufferInfo] from a Limine framebuffer
    pub fn new(framebuffer: &limine::framebuffer::Framebuffer) -> Self {
        Self {
            addr: AtomicPtr::new(framebuffer.addr().cast::<u32>()),
            width: framebuffer.width() as usize,
            height: framebuffer.height() as usize,
            pitch: framebuffer.pitch() as usize,
        }
    }

    /// Clear the entire framebuffer to a specific color.
    pub fn clear_screen(&self, color: Color) {
        for i in 0..self.height {
            for j in 0..self.width {
                let offset = self.get_offset(i, j);
                // Safety: `offset` is guaranteed to be a valid location in the framebuffer's bounds
                unsafe { *self.addr.load(Ordering::Relaxed).add(offset) = color.value() };
            }
        }
    }

    /// Draw a single pixel at the specified location if it is within the framebuffer's bounds.
    pub fn draw_pixel(&self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            let offset = self.get_offset(x, y);
            // Safety: `offset` is guaranteed to be a valid location in the framebuffer's bounds
            unsafe { *self.addr.load(Ordering::Relaxed).add(offset) = color.value() }
        }
    }

    /// Draw a character on the screen at the specified location.
    pub fn draw_char(&self, x: usize, y: usize, c: char, color: Color, font: &PsfFont) {
        let byte = c as u8;
        for row in 0..font.height() as usize {
            let glyph = font.read_glyph_row(byte as usize, row);

            for pixel in 0..font.width() as usize {
                let mask = 0x80 >> pixel;
                let bit = glyph & mask;
                if bit != 0 {
                    self.draw_pixel(x + pixel, y + row, color);
                } else {
                    self.draw_pixel(x + pixel, y + row, Color::BLACK);
                }
            }
        }
    }

    /// Get the offset into the framebuffer's memory from a given position
    fn get_offset(&self, x: usize, y: usize) -> usize {
        (y * self.pitch) / core::mem::size_of::<u32>() + x
    }
}

pub struct DevFb;

impl Device for DevFb {
    fn get_device_id(&self) -> DeviceId {
        DeviceId {
            dev_type: DeviceType::Char,
            major: 29,
            minor: 0,
        }
    }

    fn get_name(&self) -> &str {
        "fb0"
    }

    fn read(&self) -> Vec<u8> {
        let fb = framebuffer();
        let mut buf = Vec::with_capacity(fb.width * fb.height * 4);
        for i in 0..fb.height {
            for j in 0..fb.width {
                let offset = fb.get_offset(i, j);
                // Safety: `offset` is guaranteed to be a valid location in the framebuffer's bounds
                let pixel = unsafe { *fb.addr.load(Ordering::Relaxed).add(offset) };
                buf.extend_from_slice(&pixel.to_ne_bytes());
            }
        }
        buf
    }

    fn write(&self, off: usize, buf: &[u8]) -> usize {
        let fb = framebuffer();
        let mut written = 0;
        for i in (off..buf.len()).step_by(4) {
            if i + 4 > buf.len() {
                break;
            }
            let pixel = u32::from_ne_bytes([buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]);
            let x = (i / 4) % fb.width;
            let y = (i / 4) / fb.width;
            fb.draw_pixel(x, y, Color::from(pixel));
            written += 4;
        }
        written
    }
}

/// Initialize the global framebuffer from a Limine framebuffer response
pub fn init(framebuffer: &limine::response::FramebufferResponse) {
    let fb = framebuffer
        .framebuffers()
        .next()
        .expect("Attempting to get framebuffer from response");

    FRAMEBUFFER.call_once(|| Mutex::new(FramebufferInfo::new(&fb)));
    self::framebuffer().clear_screen(Color::BLACK);

    console::init();
}

/// Get the global framebufffer.
pub fn framebuffer() -> MutexGuard<'static, FramebufferInfo> {
    FRAMEBUFFER
        .get()
        .expect("Attempted to retrieve framebuffer before it was initialized")
        .lock_irq()
}
