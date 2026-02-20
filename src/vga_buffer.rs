//! vga_buffer.rs
//!
//!
//! VGA Buffer module for the kernel.
//!
//! I have revamped it from phil-opp's blog as there was a bug where you
//! couldn't change the vga font colour.
//! 
//! 

use core::fmt;

use x86_64::instructions::port::Port;

use crate::{
	lazy_static,
	utils::{mutex::SpinMutex, volatile::Volatile}
};

#[used]
#[unsafe(link_section = ".kernel_tests")]
#[unsafe(export_name = "__kernel_test_probe_vga")]
static __KERNEL_TEST_PROBE_VGA: u64 = 0x1122334455667788;

lazy_static! {
	/// A global `Writer` instance that can is used for printing to the VGA text buffer.
	///
	/// Used by the `print!` and `println!` macros.
	/// 
	/// # Safety
	/// - Buffer has to be able to point to address 0xb8000 without undefined behaviour
	pub(crate) static ref WRITER: SpinMutex<Writer> = SpinMutex::new(Writer {
		column_position: 0,
		current_row: 0,
		color_code: ColorCode::new(Color::White, Color::Black),
		buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
	});
}

/// The standard color palette in VGA text mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
	/// The Colour Black
	Black = 0,
	/// The Colour Blue
	Blue = 1,
	/// The Colour Green
	Green = 2,
	/// The Colour Cyan
	Cyan = 3,
	/// The Colour Red
	Red = 4,
	/// The Colour Magenta
	Magenta = 5,
	/// The Colour Brown
	Brown = 6,
	/// The Colour Light Gray
	LightGray = 7,
	/// The Colour Dark Gray
	DarkGray = 8,
	/// The Colour Light Blue
	LightBlue = 9,
	/// The Colour Light Green
	LightGreen = 10,
	/// The Colour Light Cyan
	LightCyan = 11,
	/// The Colour Light Red
	LightRed = 12,
	/// The Colour Pink
	Pink = 13,
	/// The Colour Yellow
	Yellow = 14,
	/// The Colour White
	White = 15
}

/// A combination of a foreground and a background color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
	/// Create a new `ColorCode` with the given foreground and background
	/// colors.
	fn new(foreground: Color, background: Color) -> ColorCode {
		ColorCode((background as u8) << 4 | (foreground as u8))
	}
}

/// A screen character in the VGA text buffer, consisting of an ASCII character
/// and a `ColorCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
	ascii_character: u8,
	color_code: ColorCode
}

impl ScreenChar {
	fn blank() -> ScreenChar {
		ScreenChar { 
			ascii_character: b' ', 
			color_code: ColorCode::new(Color::White, Color::Black) 
		}
	}

	fn new(character: char, colour_code: ColorCode) -> ScreenChar {
		ScreenChar {
			ascii_character: character as u8,
			color_code: colour_code,
		}
	}
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

/// A VGA Text Buffer
#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct Buffer {
	/// All screen characters that are currently presented on the screen.
	chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT]
}

/// A writer type that allows writing ASCII bytes and strings to an underlying
/// `Buffer`.
pub struct Writer {
	column_position: usize,
	current_row: usize,
	pub(self) color_code: ColorCode,
	buffer: &'static mut Buffer
}

impl Writer {
	/// Writes an ASCII byte to the buffer.
	fn write_byte(&mut self, byte: u8) {
		match byte {
			b'\n' => {
				self.new_line();
			}
			byte => {
				if self.column_position >= BUFFER_WIDTH {
					self.new_line();
				}

				// write at the current row (top -> down)
				let row = self.current_row;
				let col = self.column_position;

				self.buffer.chars[row][col].write(ScreenChar::new(byte as char, self.color_code));

				// advance column & update hardware cursor immediately
				self.column_position += 1;
				self.update_cursor();
			}
		}
	}

	/// Writes the given ASCII string to the buffer.
	fn write_string(&mut self, s: &str) {
		for byte in s.bytes() {
			match byte {
				// printable ASCII byte or newline
				0x20..=0x7e | b'\n' => self.write_byte(byte),
				_ => self.write_byte(0xfe)
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
		let blank = ScreenChar::blank();
		for col in 0..BUFFER_WIDTH {
			self.buffer.chars[row][col].write(blank);
		}
	}

	/// Clear the VGA buffer and screen.
	pub(crate) fn clear_everything(&mut self) {
		let blank = ScreenChar::blank();
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

	/// Update the VGA cursor to move on a character write.
	fn update_cursor(&self) {
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

	/// Copies the VGA Buffer into memory for restoration.<br>
	/// Good for applications (TUI's) where they use fullscreen and then 
	/// want to revert back to the original terminal screen.
	#[allow(dead_code)]
	pub(crate) fn copy_vga_buffer(&self) -> Buffer {
		self.buffer.clone()
	}

	/// Restores the VGA Buffer from memory. 
	/// Good for applications (TUI's) where they use fullscreen and then 
	/// want to revert back to the original terminal screen.
	#[allow(dead_code)]
	pub(crate) fn restore_vga_buffer(&mut self, prev: &Buffer) {
		for y in 0..BUFFER_HEIGHT {
			for x in 0..BUFFER_WIDTH {
				let ch = prev.chars[y][x].read();
				self.buffer.chars[y][x].write(ch);
			}
		}
	}

	/// Copies the current cursor position into memory for restoration.
	/// Good for applications (TUI's) where they use fullscreen and then 
	/// want to revert back to the original terminal screen.
	#[allow(dead_code)]
	pub(crate) fn copy_cursor_position(&self) -> (usize, usize) {
		(self.current_row, self.column_position)
	}

	/// Remove the character behind the VGA cursor.
	fn backspace(&mut self) {
		let blank = ScreenChar::blank();

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

	/// Run a closure with a temporary color, restoring the previous color
	/// afterwards.
	fn with_color<F: FnOnce(&mut Self)>(&mut self, fg: Color, bg: Color, f: F) {
		let prev = self.color_code;
		self.color_code = ColorCode::new(fg, bg);
		f(self);
		self.color_code = prev;
	}

	/// Write a whole string with the given color, then restore the previous
	/// color.
	fn write_with_color(&mut self, s: &str, fg: Color, bg: Color) {
		self.with_color(fg, bg, |w| w.write_string(s));
	}

	/// Write a sequence of segments, each segment is a tuple: (&str, fg_color,
	/// bg_color). 
	/// Example: 
	/// ```rust
	/// write_segments(&[("test", Color::Green, Color::Black), ("@nullex", Color::White, Color::Black)]);
	/// ```
	pub fn write_segments(&mut self, segments: &[(&str, Color, Color)]) {
		for (text, fg, bg) in segments {
			self.write_with_color(text, *fg, *bg);
		}
	}
}

/// Wrapper for the backspace() function of `Writer`
pub fn console_backspace() {
	WRITER.lock().backspace();
}

impl fmt::Write for Writer {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.write_string(s);
		Ok(())
	}
}

/// Like the `print!` macro in the standard library, but prints to the VGA text
/// buffer.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

/// Like the `println!` macro in the standard library, but prints to the VGA
/// text buffer.
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}


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

/// Print multiple colored segments.
///
/// Examples:
/// ```rust
/// print_colours!( ("test", Color::Green), ("@nullex", Color::White) );
/// print_colours!( ("ok: ", Color::Green), ("42\n", Color::White, Color::Blue) );
/// ```
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

#[macro_export]
/// Clear the current VGA screen of all characters. 
macro_rules! clear_screen {
	() => {
		use $crate::vga_buffer::WRITER;

		WRITER.lock().clear_everything();
	};
}

/// VGA prelude module.
pub mod prelude {
	pub use crate::vga_buffer::*;
}

#[cfg(feature = "test")]
pub mod tests {
	use crate::{utils::ktest::TestError, vga_buffer::prelude::*};

	pub fn test_screenchar_blank_and_buffer_blank() -> Result<(), TestError> {
		let sc = ScreenChar::blank();
		assert_eq!(sc.ascii_character, b' ');
		let buf = Buffer::blank();
		for row in 0..3 {
			for col in 0..3 {
				let ch = buf.chars[row][col].read();
				assert_eq!(ch.ascii_character, b' ');
			}
		}
		Ok(())
	}
	crate::create_test!(test_screenchar_blank_and_buffer_blank);

	pub fn test_color_code_creation() -> Result<(), TestError> {
		let _ = ColorCode::new(Color::LightGreen, Color::Black);
		Ok(())
	}
	crate::create_test!(test_color_code_creation);
}
