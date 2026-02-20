//! serial.rs
//!
//! Serial Interface module for the kernel.
//!

use alloc::string::String;
use core::{arch::asm, fmt, hint::spin_loop, task::Poll};

use crossbeam_queue::ArrayQueue;
use futures::{Stream, StreamExt, task::AtomicWaker};
use x86_64::instructions::interrupts;

use crate::{
	bitflags,
	common::ports::{inb, outb},
	lazy_static,
	println,
	serial_print,
	serial_println,
	serial_raw_print,
	task::yield_now,
	utils::{serial_kfunc::run_serial_command, mutex::SpinMutex, oncecell::spin::OnceCell}
};

#[derive(Debug)]
/// Structure representing a port to the Serial I/O lines.
pub struct SerialPort(u16);

impl SerialPort {
	fn port_base(&self) -> u16 {
		self.0
	}

	fn port_data(&self) -> u16 {
		self.port_base()
	}

	fn port_int_en(&self) -> u16 {
		self.port_base() + 1
	}

	fn port_fifo_ctrl(&self) -> u16 {
		self.port_base() + 2
	}

	fn port_line_ctrl(&self) -> u16 {
		self.port_base() + 3
	}

	fn port_modem_ctrl(&self) -> u16 {
		self.port_base() + 4
	}

	fn port_line_sts(&self) -> u16 {
		self.port_base() + 5
	}

	/// Creates a new `SerialPort` at the specified port / base.
	pub unsafe fn new(base: u16) -> Self {
		Self(base)
	}

	/// Initializes a new serial port for use.
	pub fn init(&mut self) {
		unsafe {
			outb(self.port_int_en(), 0x00);

			outb(self.port_line_ctrl(), 0x80);

			outb(self.port_data(), 0x03);
			outb(self.port_int_en(), 0x00);

			outb(self.port_line_ctrl(), 0x03);

			outb(self.port_fifo_ctrl(), 0xc7);

			outb(self.port_modem_ctrl(), 0x0b);

			outb(self.port_int_en(), 0x01);
		}
	}

	fn line_sts(&mut self) -> LineStatusFlags {
		unsafe { LineStatusFlags::from_bits_truncate(inb(self.port_line_sts())) }
	}

	fn send(&mut self, data: u8) {
		match data {
			8 | 0x7F => {
				self.send_raw(8);
				self.send_raw(b' ');
				self.send_raw(8);
			}
			0x0A => {
				self.send_raw(0x0D);
				self.send_raw(0x0A);
			}
			data => {
				self.send_raw(data);
			}
		}
	}

	fn send_raw(&mut self, data: u8) {
		loop {
			if let Ok(ok) = self.try_send_raw(data) {
				break ok;
			}

			spin_loop();
		}
	}

	fn try_send_raw(&mut self, data: u8) -> Result<(), SerialPortError> {
		if self.line_sts().contains(LineStatusFlags::OUTPUT_EMPTY) {
			unsafe {
				outb(self.port_data(), data);
			}
			Ok(())
		} else {
			Err(SerialPortError::SerialPortError)
		}
	}

	// this function is only here because eventually is something like kernel config, like Linux KConfig
	// someone may want serial input. so we keep here for now.
	fn _receive(&mut self) -> u8 {
		loop {
			if let Ok(ok) = self._try_receive() {
				break ok;
			}

			spin_loop();
		}
	}

	// this function is only here because eventually is something like kernel config, like Linux KConfig
	// someone may want serial input. so we keep here for now.
	fn _try_receive(&mut self) -> Result<u8, SerialPortError> {
		if self.line_sts().contains(LineStatusFlags::INPUT_FULL) {
			let data = unsafe { inb(self.port_data()) };
			Ok(data)
		} else {
			Err(SerialPortError::SerialPortError)
		}
	}
}

impl fmt::Write for SerialPort {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		for byte in s.bytes() {
			self.send(byte);
		}
		Ok(())
	}
}

// https://git.berlin.ccc.de/vinzenz/redox/src/commit/9040789987a987299ac222372c28ddb7382afb53/arch/x86_64/src/device/serial.rs
bitflags! {
	/// Line status flags
	pub struct LineStatusFlags: u8 {
		const INPUT_FULL = 1;
		const OUTPUT_EMPTY = 1 << 5;
	}
}

#[derive(thiserror::Error, Debug)]
enum SerialPortError {
	#[error("Serial Port Error.")]
	SerialPortError
}

static SERIAL_SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static SERIAL_WAKER: AtomicWaker = AtomicWaker::new();

pub(crate) fn add_byte(byte: u8) {
	if let Ok(queue) = SERIAL_SCANCODE_QUEUE.try_get() {
		if queue.push(byte).is_err() {
			println!(
				"WARNING: scancode queue full; dropping keyboard input {}",
				byte
			);
		} else {
			SERIAL_WAKER.wake();
		}
	} else {
		println!("WARNING: scancode queue uninitialized");
	}
}
// this function is only here because eventually is something like kernel config, like Linux KConfig
// someone may want serial input. so we keep here for now.
#[allow(dead_code)]
struct SerialScancodeStream {
	_private: ()
}

// this function is only here because eventually is something like kernel config, like Linux KConfig
// someone may want serial input. so we keep here for now.
#[allow(dead_code)]
impl SerialScancodeStream {
	fn new() -> Self {
		SERIAL_SCANCODE_QUEUE
			.try_init_once(|| ArrayQueue::new(1000))
			.expect("SerialScancodeStream::new should only be called once.");

		Self {
			_private: ()
		}
	}
}

impl Default for SerialScancodeStream {
	fn default() -> Self {
		Self::new()
	}
}

impl Stream for SerialScancodeStream {
	type Item = u8;

	fn poll_next(
		self: core::pin::Pin<&mut Self>,
		cx: &mut core::task::Context<'_>
	) -> core::task::Poll<Option<Self::Item>> {
		let queue = SERIAL_SCANCODE_QUEUE
			.try_get()
			.expect("SERIAL_SCANCODE_QUEUE not initialized");

		if let Some(scancode) = queue.pop() {
			return Poll::Ready(Some(scancode));
		}

		SERIAL_WAKER.register(cx.waker());

		match queue.pop() {
			Some(c) => {
				SERIAL_WAKER.take();
				Poll::Ready(Some(c))
			}
			None => Poll::Pending
		}
	}
}

lazy_static! {
	/// Static reference to the serial port `0x3F8`
	pub static ref SERIAL1: SpinMutex<SerialPort> = {
		let mut serial_port = unsafe { SerialPort::new(0x3F8) };
		serial_port.init();
		SpinMutex::new(serial_port)
	};
}

// this function is only here because eventually is something like kernel config, like Linux KConfig
// someone may want serial input. so we keep here for now.
async fn _serial_consumer_loop() -> i32 {
	let mut bytes = SerialScancodeStream::new();
	let mut line = String::new();
	// print serial terminal like ui thing
	serial_print!("serial@nullex: $ ");

	while let Some(byte) = bytes.next().await {
		if byte == 0x0A || byte == 0x0D {
			if !line.is_empty() {
				let cmd_line = line.clone();
				line.clear();
				yield_now().await;
				serial_println!();
				run_serial_command(&cmd_line);
				serial_print!("serial@nullex: $ ");
			} else {
				serial_raw_print!(b"\r\n");
				serial_print!("serial@nullex: $ ");
				line.clear();
			}

			continue;
		}

		// 7F is the main cause here, 0x08 is js there.
		if byte == 0x08 || byte == 0x7F {
			if line.is_empty() {
				serial_raw_print!(b"\x1B[1C"); // move it back so it cannot delete anything.
			}

			line.pop();
			serial_raw_print!(b"\x08 \x08");

			continue;
		}

		let c = byte as char;
		line.push(c);
		serial_print!("{}", c);
	}

	0
}

// this function is only here because eventually is something like kernel config, like Linux KConfig
// someone may want serial input. so we keep here for now.
fn _init_serial_input() {
	use x86_64::instructions::port::Port;

	interrupts::without_interrupts(|| {
		let mut port = Port::<u8>::new(0x3F9);
		let cur = unsafe { port.read() };
		let new = cur | 0x01;
		unsafe { port.write(new) };
	});

	// unmask IRQ4
	unsafe { asm!("in al, 0x21", "and al, 0xEF", "out 0x21, al",) };
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
	use core::fmt::Write;

	interrupts::without_interrupts(|| {
		SERIAL1
			.lock()
			.write_fmt(args)
			.expect("Printing to serial failed")
	});
}

#[doc(hidden)]
pub fn _send_raw_serial(bytes: &[u8]) {
	interrupts::without_interrupts(|| {
		let mut serial = SERIAL1.lock();
		for &b in bytes {
			serial.send(b);
		}
	})
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(core::format_args!($($arg)*))
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\r\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\r\n", core::format_args!($($arg)*)));
}

/// Prints to the host through the serial interface, sending raw bytes with no formatting or checking.
#[macro_export]
macro_rules! serial_raw_print {
	($bytes:expr) => {
		$crate::serial::_send_raw_serial($bytes)
	};
}

/// Prelude module for the serial I/O code.
pub mod prelude {
	pub use crate::serial::*;
}

#[cfg(feature = "test")]
pub mod tests {
	use crate::{serial::prelude::*, utils::ktest::TestError};

	pub fn test_line_status_flag_combinations() -> Result<(), TestError> {
		let flags = LineStatusFlags::INPUT_FULL | LineStatusFlags::OUTPUT_EMPTY;
		assert!(flags.contains(LineStatusFlags::INPUT_FULL));
		assert!(flags.contains(LineStatusFlags::OUTPUT_EMPTY));
		Ok(())
	}
	crate::create_test!(test_line_status_flag_combinations);

	pub fn test_serial_error_debug() -> Result<(), TestError> {
		let err = SerialPortError::SerialPortError;
		let _ = format!("{:?}", err);
		Ok(())
	}
	crate::create_test!(test_serial_error_debug);
}
