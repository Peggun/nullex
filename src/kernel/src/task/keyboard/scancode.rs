// scancode.rs

/*
Keyboard scancode handling module for the kernel.
*/

extern crate alloc;

use alloc::{
	string::{String, ToString},
	vec::Vec
};
use core::task::Poll;

use conquer_once::spin::OnceCell;
use crossbeam::queue::ArrayQueue;
use futures_util::{Stream, StreamExt, task::AtomicWaker};
use lazy_static::lazy_static;
use pc_keyboard::{HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts};
use spin::Mutex;

use crate::{
	errors::KernelError,
	errors::*, /* Assuming this contains KernelError and constants like
	            * KEYBOARD_DRIVER_NOT_INITIALIZED */
	fs,
	print,
	println,
	serial_println,
	task::{
		keyboard::commands::{CMD_HISTORY, CMD_HISTORY_INDEX},
		yield_now
	},
	vga_buffer::{WRITER, console_backspace}
};

lazy_static! {
	pub static ref CWD: Mutex<String> = Mutex::new("/".to_string());
}

#[derive(Debug, PartialEq)]
pub enum CompletionType {
	File,
	Directory,
	Both,
	None
}

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

/// Adds a scancode to the queue, returning an error if the queue is full or
/// uninitialized.
pub(crate) fn add_scancode(scancode: u8) -> Result<(), KernelError> {
	let queue = SCANCODE_QUEUE
		.try_get()
		.map_err(|_| KernelError::KeyboardError(KEYBOARD_DRIVER_NOT_INITIALIZED))?;
	queue
		.push(scancode)
		.map_err(|_| KernelError::KeyboardError(KEYBOARD_BUFFER_OVERFLOW))?;
	WAKER.wake();
	Ok(())
}

pub struct ScancodeStream {
	_private: ()
}

impl ScancodeStream {
	pub fn new() -> Self {
		SCANCODE_QUEUE
			.try_init_once(|| ArrayQueue::new(100))
			.expect("ScancodeStream::new should only be called once");
		Self {
			_private: ()
		}
	}
}

impl Stream for ScancodeStream {
	type Item = u8;

	fn poll_next(
		self: core::pin::Pin<&mut Self>,
		cx: &mut core::task::Context<'_>
	) -> Poll<Option<Self::Item>> {
		let queue = SCANCODE_QUEUE
			.try_get()
			.expect("SCANCODE_QUEUE not initialized");

		if let Some(scancode) = queue.pop() {
			return Poll::Ready(Some(scancode));
		}

		WAKER.register(&cx.waker());

		match queue.pop() {
			Some(c) => {
				WAKER.take();
				Poll::Ready(Some(c))
			}
			None => Poll::Pending
		}
	}
}

/// Processes keypresses and handles commands, returning an exit code or error.
/// Processes keypresses and handles commands, returning an exit code or error.
pub async fn print_keypresses() -> Result<i32, KernelError> {
	let mut scancodes = ScancodeStream::new();
	let mut keyboard = Keyboard::new(
		ScancodeSet1::new(),
		layouts::Us104Key,
		HandleControl::Ignore
	);
	let mut line = String::new();

	print!("test@nullex: {} $ ", *CWD.lock());
	while let Some(scancode) = scancodes.next().await {
		// Removed: add_scancode(scancode)?; - This was causing the infinite loop

		if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
			if let Some(key) = keyboard.process_keyevent(key_event) {
				match key {
					pc_keyboard::DecodedKey::RawKey(key) => {
						if key == KeyCode::LControl
							|| key == KeyCode::RControl
							|| key == KeyCode::RControl2
						{
							print!("^C\ntest@nullex: {} $ ", *CWD.lock());
							line.clear();
						} else if key == KeyCode::ArrowUp {
							uparrow_completion(&mut line);
						} else if key == KeyCode::ArrowDown {
							downarrow_completion(&mut line);
						} else {
							serial_println!("unhandled key {:?}", key);
						}
					}
					pc_keyboard::DecodedKey::Unicode(c) => {
						if c as u8 == 8 {
							// Backspace
							if !line.is_empty() {
								line.pop();
								console_backspace();
							}
							continue;
						} else if c as u8 == 27 {
							// Escape
							WRITER.lock().clear_everything();
							print!("test@nullex: {} $ ", *CWD.lock());
							continue;
						} else if c as u8 == 9 {
							// Tab
							if line.is_empty() || line.trim().is_empty() {
								line.push_str("    ");
								print!("    ");
							} else {
								tab_completion(&mut line)?;
							}
							continue;
						}

						print!("{}", c);
						if c == '\n' && !line.is_empty() {
							let command_line = line.clone();
							line.clear();
							yield_now().await;
							crate::task::keyboard::commands::run_command(&command_line)?;
							print!("test@nullex: {} $ ", *CWD.lock());
						} else {
							line.push(c);
						}
					}
				}
			}
		}
	}
	Ok(0) // Return exit code
}

/// Determines the completion type for a command.
pub fn command_supports_completion(command: &str) -> CompletionType {
	match command {
		"cd" | "ls" | "rmdir" => CompletionType::Directory,
		"cat" | "write" | "rm" => CompletionType::File,
		"clear" | "help" | "exit" | "echo" | "mkdir" | "progs" | "kill" | "touch" => {
			CompletionType::None
		}
		_ => CompletionType::Both
	}
}

/// Handles tab completion for the current input line.
pub fn tab_completion(line: &mut String) -> Result<(), KernelError> {
	let parts: Vec<&str> = line.split(' ').collect();
	let part = parts.last().unwrap_or(&"").to_string();

	let completion_type = command_supports_completion(parts.first().unwrap_or(&""));
	if completion_type == CompletionType::None {
		line.push_str("    ");
		print!("    ");
		return Ok(());
	}

	fs::with_fs(|fs| {
		let files = fs
			.list_dir(&CWD.lock())
			.map_err(|_| KernelError::FileSystemError(FS_FILE_NOT_FOUND))?;
		let file_types = fs
			.list_dir_entry_types(&CWD.lock())
			.map_err(|_| KernelError::FileSystemError(FS_FILE_NOT_FOUND))?;

		let matches: Vec<_> = files
			.iter()
			.zip(file_types.iter())
			.filter(|(f, t)| {
				f.starts_with(&part)
					&& match completion_type {
						CompletionType::File => *t == "File",
						CompletionType::Directory => *t == "Directory",
						CompletionType::Both => true,
						_ => false
					}
			})
			.map(|(f, _)| f)
			.collect();

		if matches.len() == 1 {
			let match_str = matches[0];
			for _ in 0..part.len() {
				line.pop();
				console_backspace();
			}
			line.push_str(match_str);
			print!("{}", match_str);
		} else if matches.len() > 1 {
			println!();
			for m in matches {
				println!("{}", m);
			}
			print!("test@nullex: {} $ {}", *CWD.lock(), line);
		}
		Ok(())
	})
}

/// Handles up arrow for command history navigation.
pub fn uparrow_completion(line: &mut String) {
	let history = CMD_HISTORY.lock();
	let mut index = CMD_HISTORY_INDEX.lock();

	if history.is_empty() {
		return;
	}

	if *index > 0 {
		*index -= 1;
	}

	for _ in 0..line.len() {
		line.pop();
		console_backspace();
	}

	let cmd = &history[*index];
	print!("{}", cmd);
	line.push_str(cmd);
}

/// Handles down arrow for command history navigation.
pub fn downarrow_completion(line: &mut String) {
	let history = CMD_HISTORY.lock();
	let mut index = CMD_HISTORY_INDEX.lock();

	if history.is_empty() {
		return;
	}

	if *index < history.len() - 1 {
		*index += 1;
	} else {
		for _ in 0..line.len() {
			line.pop();
			console_backspace();
		}
		return;
	}

	for _ in 0..line.len() {
		line.pop();
		console_backspace();
	}

	let cmd = &history[*index];
	print!("{}", cmd);
	line.push_str(cmd);
}
