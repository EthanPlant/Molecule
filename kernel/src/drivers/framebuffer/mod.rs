use core::sync::atomic::{AtomicPtr, Ordering};

use color::Color;
use spin::{mutex::Mutex, Once};

use crate::FRAMEBUFFER_REQUEST;

pub mod color;

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
}

pub fn framebuffer() -> spin::MutexGuard<'static, FrameBufferInfo> {
    FRAMEBUFFER
        .get()
        .expect("Framebuffer is initialized")
        .lock()
}
