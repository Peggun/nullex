// vga_buffer.rs

/*
VGA Buffer module for the kernel.

I have revamped it from phil-opp's blog as there was a bug where you
couldnt change the vga font colour.
*/

use core::{array::from_fn, fmt};
use lazy_static::lazy_static;
use spin::Mutex;
use volatile::Volatile;
use x86_64::instructions::port::Port;

lazy_static! {
    /// A global `Writer` instance that can be used for printing to the VGA text buffer.
    ///
    /// Used by the `print!` and `println!` macros.
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        current_row: 0,
        // Make the font colour white on black by default:
        color_code: ColorCode::new(Color::White, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

/// The standard color palette in VGA text mode.
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

/// A combination of a foreground and a background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    /// Create a new `ColorCode` with the given foreground and background colors.
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

/// A screen character in the VGA text buffer, consisting of an ASCII character and a `ColorCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

impl ScreenChar {
    pub fn blank() -> Self {
        ScreenChar {
            ascii_character: b' ',
            color_code: ColorCode::new(Color::White, Color::Black),
        }
    }
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

/// A structure representing the VGA text buffer.
#[derive(Clone)]
#[repr(transparent)]
pub struct Buffer {
    pub chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

impl Buffer {
    pub fn blank() -> Self {
        let chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT] =
            from_fn(|_| from_fn(|_| Volatile::new(ScreenChar::blank())));

        Buffer { chars }
    }
}

/// A writer type that allows writing ASCII bytes and strings to an underlying `Buffer`.
///
/// Wraps lines at `BUFFER_WIDTH`. Supports newline characters and implements the
/// `core::fmt::Write` trait.
pub struct Writer {
    pub column_position: usize,
    pub current_row: usize,
    pub color_code: ColorCode,
    pub buffer: &'static mut Buffer,
}

#[derive(Debug, Clone, Copy)]
pub struct BufferEntry {
    pub character: u8,
    pub colour_code: u8,
}

impl Writer {
	pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.color_code = ColorCode::new(fg, bg);
    }

    /// Writes an ASCII byte to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character.
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.new_line();
            }
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                // write at the current row (top → down)
                let row = self.current_row;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });

                // advance column & update hardware cursor immediately
                self.column_position += 1;
                self.update_cursor();
            }
        }
    }

    /// Writes the given ASCII string to the buffer.
    ///
    /// Wraps lines at `BUFFER_WIDTH`. Supports the `\n` newline character. Does **not**
    /// support strings with non-ASCII characters, since they can't be printed in the VGA text
    /// mode.
    fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }
        }
    }

    /// Shifts lines up when the buffer is full and moves to the next line.
    fn new_line(&mut self) {
        self.current_row += 1;

        if self.current_row >= BUFFER_HEIGHT {
            // scroll up
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let character = self.buffer.chars[row][col].read();
                    self.buffer.chars[row - 1][col].write(character);
                }
            }
            // clear last line
            self.clear_row(BUFFER_HEIGHT - 1);
            self.current_row = BUFFER_HEIGHT - 1;
        }

        self.column_position = 0;
        self.update_cursor();
    }

    /// Clears a row by overwriting it with blank characters.
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    pub fn clear_everything(&mut self) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for row in 0..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                self.buffer.chars[row][col].write(blank);
            }
        }
        // reset cursor to top-left after clearing
        self.current_row = 0;
        self.column_position = 0;
        self.update_cursor();
    }

    pub fn update_cursor(&self) {
        // hardware cursor position = row * width + col
        let position = (self.current_row * BUFFER_WIDTH) + self.column_position;

        let mut port_3d4 = Port::<u8>::new(0x3D4);
        let mut port_3d5 = Port::<u8>::new(0x3D5);
        unsafe {
            port_3d4.write(0x0F);
            port_3d5.write((position & 0xFF) as u8);
            port_3d4.write(0x0E);
            port_3d5.write(((position >> 8) & 0xFF) as u8);
        }
    }

    pub fn copy_vga_buffer(&self) -> Buffer {
        self.buffer.clone()
    }

    pub fn restore_vga_buffer(&mut self, prev: &Buffer) {
        for y in 0..BUFFER_HEIGHT {
            for x in 0..BUFFER_WIDTH {
                let ch = prev.chars[y][x].read();
                self.buffer.chars[y][x].write(ch);
            }
        }
    }

    pub fn copy_cursor_position(&self) -> (usize, usize) {
        (self.current_row, self.column_position)
    }

    pub fn backspace(&mut self) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };

        if self.column_position == 0 {
            if self.current_row == 0 {
                return;
            }
            // move up a row and to end of the previous line
            self.current_row -= 1;
            self.column_position = BUFFER_WIDTH - 1;
        } else {
            self.column_position -= 1;
        }

        // write blank at the new cursor position and update the cursor
        self.buffer.chars[self.current_row][self.column_position].write(blank);
        self.update_cursor();
    }

	/// Run a closure with a temporary color, restoring the previous color afterwards.
	pub fn with_color<F: FnOnce(&mut Self)>(&mut self, fg: Color, bg: Color, f: F) {
        let prev = self.color_code;
        self.color_code = ColorCode::new(fg, bg);
        f(self);
        self.color_code = prev;
    }

    /// Write a whole string with the given color, then restore the previous color.
    pub fn write_with_color(&mut self, s: &str, fg: Color, bg: Color) {
        self.with_color(fg, bg, |w| w.write_string(s));
    }

    /// Write a sequence of segments, each segment is a tuple: (&str, fg_color, bg_color).
    /// Example: write_segments(&[("test", Color::Green, Color::Black), ("@nullex", Color::White, Color::Black)]);
    pub fn write_segments(&mut self, segments: &[(&str, Color, Color)]) {
        for (text, fg, bg) in segments {
            self.write_with_color(text, *fg, *bg);
        }
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

/// Like the `print!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the VGA text buffer.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

/// Prints the given formatted string to the VGA text buffer through the global `WRITER` instance.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

#[doc(hidden)]
pub fn _print_segments(segments: &[(&str, Color, Color)]) {
    let mut w = WRITER.lock();
    w.write_segments(segments);
}

/// Print multiple colored segments. Each segment is a tuple:
///   ("text", Color::SomeColor)                      -> background defaults to Color::Black
///   ("text", Color::SomeColor, Color::OtherColor)  -> explicit background
///
/// Examples:
///   print_colours!( ("test", Color::Green), ("@nullex", Color::White) );
///   print_colours!( ("ok: ", Color::Green), ("42\n", Color::White, Color::Blue) );
#[macro_export]
macro_rules! print_colours {
    // main entry: one-or-more segments (each segment: (text, fg [, bg]))
    ( $( ($text:expr, $fg:expr $(, $bg:expr)? ) ),+ $(,)? ) => {
        {
            // bring helper into scope
            use $crate::vga_buffer::{_print_segments, Color};

            // build a literal slice of segments. Color::black is default background.
            let segments: &[(&str, Color, Color)] = &[
                $(
                    ($text, $fg, $crate::print_colours!(@bg_or $($bg)?)),
                )+
            ];

            _print_segments(segments);
        }
    };

    (@bg_or $bg:expr) => { $bg };
    (@bg_or) => { $crate::vga_buffer::Color::Black };
}