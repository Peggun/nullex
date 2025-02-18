use alloc::{borrow::ToOwned, string::{String, ToString}, vec::Vec};
// vga_buffer.rs
use volatile::Volatile;
use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;
use x86_64::instructions::{interrupts, port::Port};

use crate::{println, print};

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    interrupts::without_interrupts(|| {
        let mut writer = WRITER.lock();
        // Clear the screen and reset positions to ensure a clean slate.
        writer.clear_screen();
        writer.column_position = 0;
        writer.row_position = 0;
        writer.input_start_column = 0;
        writer.input_start_row = 0;

        // Print the string without an extra leading newline.
        writeln!(writer, "{}", "Some test string that fits on a single line")
            .expect("writeln failed");

        // Now, the string should be exactly at row 0.
        let s = "Some test string that fits on a single line";
        for (i, c) in s.chars().enumerate() {
            let screen_char = writer.buffer.chars[0][i].read();
            assert_eq!(char::from(screen_char.ascii_character), c);
        }
    });
}


lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        row_position: 0,
        input_start_column: 0,
        input_start_row: 0, // NEW
        color_code: ColorCode::new(Color::White, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    pub ascii_character: u8,
    pub color_code: ColorCode,
}

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
pub struct Buffer {
    pub chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    pub column_position: usize,
    pub row_position: usize,
    pub input_start_column: usize,
    pub input_start_row: usize, // NEW FIELD
    pub color_code: ColorCode,
    pub buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            b'\x08' => self.backspace(), // Backspace character
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = self.row_position; // USE row_position
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
        self.set_cursor_pos(); // Update cursor after each byte write
    }

    pub fn backspace(&mut self) {
        if self.row_position > self.input_start_row || 
           (self.row_position == self.input_start_row && 
            self.column_position > self.input_start_column) {
            if self.column_position > 0 {
                self.column_position -= 1;
            } else {
                self.row_position -= 1;
                self.column_position = BUFFER_WIDTH - 1;
            }
            // Clear the character
            let blank = ScreenChar {
                ascii_character: b' ',
                color_code: self.color_code,
            };
            self.buffer.chars[self.row_position][self.column_position].write(blank);
            self.set_cursor_pos();
        }
    }


    pub fn new_line(&mut self) {
        self.column_position = 0;
        self.row_position += 1;
        if self.row_position >= BUFFER_HEIGHT {
            // Scroll up (shift rows upwards)
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let character = self.buffer.chars[row][col].read();
                    self.buffer.chars[row - 1][col].write(character);
                }
            }
            self.clear_row(BUFFER_HEIGHT - 1);
            self.row_position = BUFFER_HEIGHT - 1; // Continue writing on the last line after scroll.
            // If you want to start from the top line again after scrolling, use:
            // self.row_position = 0;
        }
    }

    pub fn clear_screen(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);
        }
        self.row_position = 0;
        self.column_position = 0;
        self.input_start_column = 0; // Reset input_start_column on clear screen, if needed
        self.set_cursor_pos(); // Update cursor after clear screen
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                0x20..=0x7e | b'\n' | b'\x08' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    // Sets the hardware cursor position
    pub fn set_cursor_pos(&mut self) {
        let pos = self.row_position * BUFFER_WIDTH + self.column_position;

        // VGA cursor control ports
        let mut port_command = Port::<u8>::new(0x3D4);
        let mut port_data = Port::<u8>::new(0x3D5);

        // Send command to set the high byte of the cursor position
        unsafe {
            port_command.write(0x0E);
            port_data.write((pos >> 8) as u8); // High byte
            // Send command to set the low byte of the cursor position
            port_command.write(0x0F);
            port_data.write((pos & 0xff) as u8); // Low byte
        }
    }
}


impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

pub fn print_something() {
    use core::fmt::Write;
    let mut writer = Writer {
        column_position: 0,
        row_position: 0,
        input_start_column: 0,
        input_start_row: 0, // NEW
        color_code: ColorCode::new(Color::White, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    };

    writer.write_byte(b'H');
    writer.write_string("ello! ");
    write!(writer, "The numbers are {} and {}", 42, 1.0/3.0).unwrap();
}

// ----- MACROS ----- //
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! printnl {
    () => ($crate::print!("\n"));
}


#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;   // new
    use lazy_static::initialize;

    interrupts::without_interrupts(|| {     // new
        let mut writer = WRITER.lock();
        let initial_column = writer.column_position; // Get column position before writing
        writer.write_fmt(args).unwrap();

        // Set input_start_column right after printing the initial prompt, only if it's the very first print
        if writer.row_position == 0 && initial_column == 0 { // Check if we are at the beginning of first row
            writer.input_start_column = writer.column_position;
        }
    });
}

pub fn clear_screen() {
    interrupts::without_interrupts(|| {
        WRITER.lock().clear_screen();
    });
}