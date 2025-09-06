// serial.rs

/*
Serial Interface module for the kernel.
*/

use core::arch::asm;
use core::task::Poll;
use alloc::string::String;
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures::StreamExt;
use futures::{task::AtomicWaker, Stream};
use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;
use x86_64::instructions::interrupts;

use crate::println;
use crate::serial_print;
use crate::serial_println;
use crate::serial_raw_print;
use crate::task::yield_now;
use crate::utils::kfunc::run_serial_command;

static SERIAL_SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static SERIAL_WAKER: AtomicWaker = AtomicWaker::new();

pub(crate) fn add_byte(byte: u8) {
	if let Ok(queue) = SERIAL_SCANCODE_QUEUE.try_get() {
		if let Err(_) = queue.push(byte) {
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

pub struct SerialScancodeStream {
	_private: ()
}

impl SerialScancodeStream {
	pub fn new() -> Self {
		SERIAL_SCANCODE_QUEUE
    		.try_init_once(|| ArrayQueue::new(1000))
    		.expect("SerialScancodeStream::new should only be called once.");

		Self {
			_private: ()
		}
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

		SERIAL_WAKER.register(&cx.waker());

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
	pub static ref SERIAL1: Mutex<SerialPort> = {
		let mut serial_port = unsafe { SerialPort::new(0x3F8) };
		serial_port.init();
		Mutex::new(serial_port)
	};
}

pub async fn serial_consumer_loop() -> i32 {
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

pub fn init_serial_input() {
	use x86_64::instructions::port::Port;

	interrupts::without_interrupts(|| {
		let mut port = Port::<u8>::new(0x3F9);
		let cur = unsafe { port.read() };
		let new = cur | 0x01;
		unsafe { port.write(new) };
	});

	// unmask IRQ4
	unsafe {
		asm!(
			"in al, 0x21",
			"and al, 0xEF",
			"out 0x21, al",
		)
	};
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
			serial.send_raw(b);
		}
	})
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(core::format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\r\n"));
    ($($arg:tt)*) => ($crate::serial_print!("{}\r\n", core::format_args!($($arg)*)));
}

#[macro_export]
macro_rules! serial_raw_print {
	($bytes:expr) => {
		$crate::serial::_send_raw_serial($bytes)
	}
}