use core::fmt::{self, Write};
use vga::{
    colors::{Color16, TextModeColor, DEFAULT_PALETTE},
    fonts::TEXT_8X16_FONT,
    vga::VGA,
    writers::{ScreenCharacter, Text80x25, TextWriter},
};
use volatile::Volatile;

use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::instructions::{interrupts, port::Port};

use crate::{serial_print, serial_println};

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer::new());
    // pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
    //     column_position: 0,
    //     color_code: ColorCode::new(Color::White, Color::Black),
    //     buffer: unsafe { &mut *(0xb8000 as *mut Buffer) }
    // });
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

pub fn string_to_color(s: &str) -> Option<Color16> {
    let color = match s {
        "red" => Color16::Red,
        "blue" => Color16::Blue,
        "green" => Color16::Green,
        "cyan" => Color16::Cyan,
        "brown" => Color16::Brown,
        "magenta" => Color16::Magenta,
        "pink" => Color16::Pink,
        "yellow" => Color16::Yellow,
        "white" => Color16::White,
        "black" => Color16::Black,
        _ => {
            return None;
        }
    };
    Some(color)
}

impl Color {
    pub fn from_string(s: &str) -> Option<Color> {
        let color = match s {
            "red" => Self::Red,
            "blue" => Self::Blue,
            "green" => Self::Green,
            "cyan" => Self::Cyan,
            "brown" => Self::Brown,
            "magenta" => Self::Magenta,
            "pink" => Self::Pink,
            "yellow" => Self::Yellow,
            "white" => Self::White,
            "black" => Self::Black,
            _ => {
                return None;
            }
        };
        Some(color)
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    pub fn new(foreground: Color, background: Color) -> Self {
        Self((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    //buffer: &'static mut Buffer,
    text: Text80x25,
    color: TextModeColor,
    pub current_row: usize
}

impl Writer {
    pub fn set_colors(&mut self, fg: Color16, bg: Color16) {
        self.color = TextModeColor::new(fg, bg);
    }

    pub fn new() -> Self {
        let text = Text80x25::new();
        text.set_mode();
        // Initialize VGA settings for correct palette and font
        {
            let mut vga = VGA.lock();
            vga.set_video_mode(vga::vga::VideoMode::Mode80x25);
            vga.color_palette_registers.load_palette(&DEFAULT_PALETTE);
            vga.load_font(&TEXT_8X16_FONT);
        }
        Self {
            color: TextModeColor::new(Color16::White, Color16::Black), // White text on black
            color_code: ColorCode::new(Color::White, Color::Black),
            text,
            column_position: 0,
            current_row: 0, // Start at top row
        }
    }

    fn update_cursor(&mut self) {
        let position = self.current_row * BUFFER_WIDTH + self.column_position;
        let mut port_3d4 = Port::<u8>::new(0x3D4);
        let mut port_3d5 = Port::<u8>::new(0x3D5);
        unsafe {
            port_3d4.write(0x0F);
            port_3d5.write((position & 0xFF) as u8);
            port_3d4.write(0x0E);
            port_3d5.write(((position >> 8) & 0xFF) as u8);
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // Printable ASCII bytes or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => {
                    self.write_byte(0xfe);
                    serial_println!("unknown key pressed {}", byte);
                }
            };
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                // Write to current_row instead of the bottom row
                self.text.write_character(
                    self.column_position,
                    self.current_row,
                    ScreenCharacter::new(byte, self.color),
                );
                self.column_position += 1;
                self.update_cursor();
            }
        }
    }

    pub fn new_line(&mut self) {
        self.current_row += 1;
        if self.current_row >= BUFFER_HEIGHT {
            // Scroll all lines up by one row
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let character = self.text.read_character(col, row);
                    self.text.write_character(col, row - 1, character);
                }
            }
            // Clear the bottom row and reset current_row
            self.clear_row(BUFFER_HEIGHT - 1);
            self.current_row = BUFFER_HEIGHT - 1;
        }
        self.column_position = 0;
        self.update_cursor();
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenCharacter::new(b' ', self.color);
        for col in 0..BUFFER_WIDTH {
            self.text.write_character(col, row, blank);
        }
    }

    pub fn backspace(&mut self) {
        let blank = ScreenCharacter::new(b' ', self.color);
    
        if self.column_position == 0 {
            if self.current_row == 0 {
                return; // Already at top-left, can't backspace
            }
            self.current_row -= 1;
            self.column_position = BUFFER_WIDTH - 1;
        } else {
            self.column_position -= 1;
        }
    
        // Clear the character at the new position
        self.text.write_character(self.column_position, self.current_row, blank);
        self.update_cursor();
    }

    pub fn clear_everything(&mut self) {
        self.text.set_mode();
        // {
        //     let mut vga = VGA.lock();
        //     vga.set_video_mode(vga::vga::VideoMode::Mode80x25);
        //     vga.color_palette_registers.load_palette(&DEFAULT_PALETTE);

        //     vga.load_font(&TEXT_8X16_FONT);
        // }
        self.text.clear_screen();
        self.column_position = 0;
        self.current_row = 0;
        self.update_cursor();
    }
}

pub fn console_backspace() {
    WRITER.lock().backspace();
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(core::format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", core::format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    })
}