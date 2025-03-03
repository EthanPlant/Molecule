use core::{
    ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

use color::Color;
use spin::Once;

use crate::{
    drivers::uart_16650::serial_println,
    logger,
    psf::PsfFont,
    sync::{Mutex, MutexGuard},
    FRAMEBUFFER_REQUEST,
};

pub mod color;
pub mod console;

pub static FRAMEBUFFER: Once<Mutex<FrameBufferInfo>> = Once::new();

/// In-memory representation of the framebuffer's data.
pub struct FrameBufferInfo {
    addr: AtomicPtr<u32>,
    width: usize,
    height: usize,
    pitch: usize,
}

impl FrameBufferInfo {
    /// Create a new `FrameBufferInfo` from a limine framebuffer.
    pub fn new(framebuffer: &limine::framebuffer::Framebuffer) -> Self {
        Self {
            #[allow(clippy::cast_ptr_alignment)]
            addr: AtomicPtr::new(framebuffer.addr().cast::<u32>()),
            width: framebuffer.width() as usize,
            height: framebuffer.height() as usize,
            pitch: framebuffer.pitch() as usize,
        }
    }

    /// Clear the entire screen to a single color
    pub fn clear_screen(&self, color: Color) {
        for y in 0..self.height {
            for x in 0..self.width {
                let offset = (y * self.pitch) / 4 + x;
                // Safety: The offset is guaranteed to be a valid location in the framebuffer
                unsafe {
                    *self.addr.load(Ordering::Relaxed).add(offset) = color.value();
                }
            }
        }
    }

    /// Draw a single pixel at the specified location
    /// If the pixel location is outside of the framebuffer, nothing occurs.
    pub fn draw_pixel(&self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            let offset = (y * self.pitch) / 4 + x;
            // Safety: The offset is guaranteed to be a valid location in the framebuffer
            unsafe {
                *self.addr.load(Ordering::Relaxed).add(offset) = color.value();
            }
        }
    }

    /// Draw a character on the screen at the specified location
    pub fn draw_char(&self, x: usize, y: usize, color: Color, c: char, font: &PsfFont) {
        let b = c as u8;
        for row in 0..font.height() as usize {
            let glyph = font.read_glyph_row(b as usize, row);

            for pixel in 0..font.width() as usize {
                let mask = 0x80 >> pixel;
                let bit = glyph & mask;
                if bit != 0 {
                    self.draw_pixel(x + pixel, y + row, color);
                }
            }
        }
    }

    /// Draw a string of text onto the screen at the specified location
    pub fn draw_string(&self, x: usize, y: usize, color: Color, s: &str, font: &PsfFont) {
        for (i, c) in s.chars().enumerate() {
            self.draw_char(x + i * font.width() as usize + 1, y, color, c, font);
        }
    }

    pub fn scroll(&self, scroll_height: usize) {
        assert!(scroll_height < self.height);
        unsafe {
            let scroll_size = self.pitch * scroll_height;
            let dest = self.addr.load(Ordering::Acquire);
            let src = dest.add(scroll_size / 4);

            let size = (self.pitch * self.height).saturating_sub(scroll_size) / 4;
            ptr::copy(src, dest, size);

            let clear = dest.add(size);
            ptr::write_bytes(clear, 0x00, scroll_size);
        }
    }
}

pub fn init() {
    let fb_resp = FRAMEBUFFER_REQUEST
        .get_response()
        .expect("No framebuffer response from Limine");

    FRAMEBUFFER.call_once(|| {
        Mutex::new(FrameBufferInfo::new(
            &fb_resp
                .framebuffers()
                .next()
                .expect("No framebuffer returned from Limine"),
        ))
    });

    logger::set_console_debug(true);
}

pub fn framebuffer() -> MutexGuard<'static, FrameBufferInfo> {
    FRAMEBUFFER
        .get()
        .expect("Framebuffer is initialized")
        .lock_irq()
}
