//! Abstractions around a framebuffer console for printing text.

use alloc::vec::Vec;
use core::fmt::{self, Write};

use spin::Once;

use super::color::Color;
use super::{framebuffer, FramebufferInfo};
use crate::logger;
use crate::psf::PsfFont;
use crate::sync::Mutex;

/// The global console.
static CONSOLE: Once<Mutex<Console>> = Once::new();

/// ANSI control code parsing state
struct ControlState {
    buffer: [char; 2],
    index: u8,
    color: Color,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            buffer: ['\0'; 2],
            index: 0,
            color: Color::WHITE,
        }
    }
}

/// A single character in the console.
#[derive(Clone, Copy)]
struct ConsoleChar {
    c: char,
    color: Color,
}

impl Default for ConsoleChar {
    fn default() -> Self {
        Self {
            c: '\0',
            color: Color::BLACK,
        }
    }
}

/// In memory representation of the console.
struct Console {
    font: PsfFont,
    cursor_x: usize,
    cursor_y: usize,
    width: usize,
    height: usize,
    buffer: Vec<ConsoleChar>,
    control_mode: bool,
    control_state: ControlState,
}

impl Console {
    /// Create a new console from a framebuffer.
    pub fn new(framebuffer: &FramebufferInfo) -> Self {
        let font = PsfFont::default();
        let width = framebuffer.width / font.width() as usize;
        let height = framebuffer.height / font.height() as usize;

        Self {
            font,
            cursor_x: 0,
            cursor_y: 0,
            width,
            height,
            buffer: alloc::vec![ConsoleChar::default(); width * height],
            control_mode: false,
            control_state: ControlState::default(),
        }
    }

    /// Write a single character to the console
    pub fn write_char(&mut self, c: char) {
        if self.control_mode {
            self.parse_ansi(c);
        } else {
            match c {
                '\n' => self.newline(),
                '\x1b' => self.control_mode = true,
                _ => {
                    self.buffer[self.cursor_y * self.width + self.cursor_x] = ConsoleChar {
                        c,
                        color: self.control_state.color,
                    };
                    self.update_cursor();
                }
            }
        }
    }

    /// Update the cursor location.
    fn update_cursor(&mut self) {
        self.cursor_x += 1;
        if self.cursor_x >= self.width {
            self.newline();
        }
    }

    /// Move to the next line.
    fn newline(&mut self) {
        self.cursor_x = 0;
        self.cursor_y += 1;
        if self.cursor_y >= self.height {
            self.scroll();
        }

        self.flush();
    }

    /// Scroll the console.
    fn scroll(&mut self) {
        for y in 1..self.height {
            for x in 0..self.width {
                self.buffer[(y - 1) * self.width + x] = self.buffer[y * self.width + x];
            }
        }

        for x in 0..self.width {
            self.buffer[(self.height - 1) * self.width + x] = ConsoleChar {
                c: '\0',
                color: Color::BLACK,
            }
        }

        self.cursor_y -= 1;
        self.flush();
    }

    /// Flush the contents of the console buffer to the console
    fn flush(&self) {
        for i in 0..self.height {
            for j in 0..self.width {
                framebuffer().draw_char(
                    j * self.font.width() as usize + 1,
                    i * self.font.height() as usize + 1,
                    self.buffer[i * self.width + j].c,
                    self.buffer[i * self.width + j].color,
                    &self.font,
                );
            }
        }
    }

    /// Parse a character in an ANSI escape sequence.
    fn parse_ansi(&mut self, c: char) {
        match c {
            '[' => self.control_state.index = 0,
            '0'..='9' => {
                if self.control_state.index > 1 {
                    self.control_mode = false;
                    return;
                }

                self.control_state.buffer[self.control_state.index as usize] = c;
                self.control_state.index += 1;
            }
            ';' => {
                if self.control_state.index == 2 {
                    self.set_color()
                }
                if self.control_state.index == 1 {
                    self.graphics_command();
                }

                self.control_state.index = 0;
            }
            'm' => {
                if self.control_state.index == 2 {
                    self.set_color();
                }

                if self.control_state.index == 1 {
                    self.graphics_command();
                }

                self.control_mode = false;
                self.control_state.index = 0;
            }
            _ => {
                self.control_mode = false;
                self.control_state.index = 0;
            }
        }
    }

    /// Set the color from an ANSI escape sequence.
    fn set_color(&mut self) {
        match self.control_state.buffer[0] {
            '3' => match self.control_state.buffer[1] {
                '0' => self.control_state.color = Color::BLACK,
                '1' => self.control_state.color = Color::RED,
                '2' => self.control_state.color = Color::GREEN,
                '3' => self.control_state.color = Color::YELLOW,
                '4' => self.control_state.color = Color::BLUE,
                '5' => self.control_state.color = Color::MAGENTA,
                '6' => self.control_state.color = Color::CYAN,
                '7' => self.control_state.color = Color::WHITE,
                _ => unreachable!("Invalid ANSI color code"),
            },
            _ => unimplemented!("Unsupported ANSI color code"),
        }
    }

    /// Perform an ANSI graphics command
    fn graphics_command(&mut self) {
        match self.control_state.buffer[0] {
            '0' => self.control_state.color = Color::WHITE,
            _ => unimplemented!("Invalid ANSI graphics command"),
        }
    }
}

impl fmt::Write for Console {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for char in s.chars() {
            self.write_char(char);
        }
        Ok(())
    }
}

/// Initialize the console from the global framebuffer
pub fn init() {
    CONSOLE.call_once(|| Mutex::new(Console::new(&framebuffer())));
    logger::set_console_debug(true);
    log::info!("Framebuffer console initialized");
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
        .get()
        .expect("Attempted to write to console before it was initialized")
        .lock_irq()
        .write_fmt(args)
        .unwrap();
}
