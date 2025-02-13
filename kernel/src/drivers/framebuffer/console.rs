use core::fmt::Write;

use alloc::fmt;
use spin::{Lazy, Mutex};

use crate::psf::PsfFont;

use super::{color::Color, framebuffer};

pub static CONSOLE: Lazy<Mutex<Console>> = Lazy::new(|| Mutex::new(Console::default()));

pub struct Console {
    font: PsfFont,
    cursor_x: usize,
    cursor_y: usize,
    color: Color,
}

impl Console {
    pub fn write_char(&mut self, c: char) {
        match c {
            '\n' => self.newline(),
            _ => {
                framebuffer().draw_char(
                    self.cursor_x * self.font.width() as usize + 1,
                    self.cursor_y + self.font.height() as usize + 1,
                    self.color,
                    c,
                    &self.font,
                );
            }
        }
        self.inc_col();
    }

    pub fn write_str(&mut self, str: &str) {
        for char in str.chars() {
            self.write_char(char);
        }
    }

    fn get_wrap_pos(&self) -> usize {
        framebuffer().pitch / self.font.width() as usize / 4
    }

    fn inc_col(&mut self) {
        self.cursor_x += 1;
        if self.cursor_x >= self.get_wrap_pos() {
            self.newline();
        }
    }

    fn newline(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += 1;
    }
}

impl Default for Console {
    fn default() -> Self {
        Self {
            font: PsfFont::default(),
            cursor_x: 0,
            cursor_y: 0,
            color: Color::WHITE,
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

pub macro print($($arg:tt)*) {
    ($crate::drivers::framebuffer::console::print_internal(format_args!($($arg)*)))
}

pub macro println {
    () => ($crate::drivers::framebuffer::console::print!("\n")),
    ($($arg:tt)*) => ($crate::drivers::framebuffer::console::print!("{}\n", format_args!($($arg)*))),
}

#[doc(hidden)]
pub fn print_internal(args: fmt::Arguments) {
    CONSOLE
        .lock()
        .write_fmt(args)
        .expect("Console writing can not fail");
}
