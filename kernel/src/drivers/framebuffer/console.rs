use core::fmt::Write;

use alloc::fmt;
use spin::Lazy;

use crate::{drivers::uart_16650::serial_println, psf::PsfFont, sync::Mutex};

use super::{color::Color, framebuffer, FRAMEBUFFER};

pub static CONSOLE: Lazy<Mutex<Console>> = Lazy::new(|| Mutex::new(Console::default()));

struct ControlState {
    color_buffer: [char; 2],
    color_index: u8,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            color_buffer: ['\0', '\0'],
            color_index: 0,
        }
    }
}

pub struct Console {
    font: PsfFont,
    cursor_x: usize,
    cursor_y: usize,
    color: Color,
    control_mode: bool,
    control_state: ControlState,
}

impl Console {
    pub fn write_char(&mut self, c: char) {
        if self.control_mode {
            match c {
                '[' => self.control_state.color_index = 0,
                '0'..='9' => {
                    if self.control_state.color_index > 1 {
                        self.control_mode = false;
                        return;
                    }

                    self.control_state.color_buffer[self.control_state.color_index as usize] = c;
                    self.control_state.color_index += 1;
                }
                ';' => {
                    if self.control_state.color_index == 2 {
                        self.set_color();
                    }
                    if self.control_state.color_index == 1 {
                        self.graphics_command();
                    }

                    self.control_state.color_index = 0;
                }
                'm' => {
                    if self.control_state.color_index == 2 {
                        self.set_color();
                    }
                    if self.control_state.color_index == 1 {
                        self.graphics_command();
                    }

                    self.control_mode = false;
                    self.control_state.color_index = 0;
                }
                _ => {
                    self.control_mode = false;
                    self.control_state.color_index = 0;
                }
            }
        } else {
            match c {
                '\n' => self.newline(),
                '\x1b' => self.control_mode = true,
                _ => {
                    framebuffer().draw_char(
                        self.cursor_x * self.font.width() as usize + 1,
                        self.cursor_y * self.font.height() as usize + 1,
                        self.color,
                        c,
                        &self.font,
                    );
                    self.inc_col();
                }
            }
        }
    }

    pub fn write_str(&mut self, str: &str) {
        for char in str.chars() {
            self.write_char(char);
        }
    }

    fn get_wrap_pos(&self) -> usize {
        framebuffer().width / self.font.width() as usize
    }

    fn get_scroll_pos(&self) -> usize {
        framebuffer().height / self.font.height() as usize
    }

    fn inc_col(&mut self) {
        self.cursor_x += 1;
        if self.cursor_x >= self.get_wrap_pos() {
            self.newline();
        }
    }

    fn inc_row(&mut self) {
        self.cursor_y += 1;
        if self.cursor_y == self.get_scroll_pos() {
            self.scroll();
        }
    }

    fn scroll(&mut self) {
        self.cursor_y -= 1;
        framebuffer().scroll(self.font.height() as usize);
    }

    fn newline(&mut self) {
        self.cursor_x = 0;
        self.inc_row();
    }

    fn set_color(&mut self) {
        match self.control_state.color_buffer[0] {
            '3' => match self.control_state.color_buffer[1] {
                '0' => self.color = Color::BLACK,
                '1' => self.color = Color::RED,
                '2' => self.color = Color::GREEN,
                '3' => self.color = Color::YELLOW,
                '4' => self.color = Color::BLUE,
                '5' => self.color = Color::MAGENTA,
                '6' => self.color = Color::CYAN,
                '7' => self.color = Color::WHITE,
                '9' => self.color = Color::WHITE,
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    fn graphics_command(&mut self) {
        match self.control_state.color_buffer[0] {
            '0' => self.color = Color::WHITE,
            _ => unreachable!(),
        }
    }
}

impl Default for Console {
    fn default() -> Self {
        Self {
            font: PsfFont::default(),
            cursor_x: 0,
            cursor_y: 0,
            color: Color::WHITE,
            control_mode: false,
            control_state: ControlState::default(),
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
        .lock_irq()
        .write_fmt(args)
        .expect("Console writing can not fail");
}
