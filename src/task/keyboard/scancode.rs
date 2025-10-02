// scancode.rs

/*
Keyboard scancode handling module for the kernel.
*/

/*
NOTE: I gotta refactor this code. Lots of if-else statements and match statements
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
	fs,
	print,
	println,
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

pub(crate) fn add_scancode(scancode: u8) {
	if let Ok(queue) = SCANCODE_QUEUE.try_get() {
		if queue.push(scancode).is_err() {
			println!(
				"WARNING: scancode queue full; dropping keyboard input {}",
				scancode
			);
		} else {
			WAKER.wake();
		}
	} else {
		println!("WARNING: scancode queue uninitialized");
	}
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

impl Default for ScancodeStream {
	fn default() -> Self {
		Self::new()
	}
}

impl Stream for ScancodeStream {
	type Item = u8;

	fn poll_next(
		self: core::pin::Pin<&mut Self>,
		cx: &mut core::task::Context<'_>
	) -> core::task::Poll<Option<Self::Item>> {
		let queue = SCANCODE_QUEUE
			.try_get()
			.expect("SCANCODE_QUEUE not initialized");

		if let Some(scancode) = queue.pop() {
			return Poll::Ready(Some(scancode));
		}

		WAKER.register(cx.waker());

		match queue.pop() {
			Some(c) => {
				WAKER.take();
				Poll::Ready(Some(c))
			}
			None => Poll::Pending
		}
	}
}

/// The async function that reads scancodes and processes keypresses.
/// Notice that when a full command line is ready on newline,
/// we yield before calling run_command so that any locks like the VGA lock
/// used during key echoing have been released.
pub async fn print_keypresses() -> i32 {
	let mut scancodes = ScancodeStream::new();

	let mut keyboard = Keyboard::new(
		ScancodeSet1::new(),
		layouts::Us104Key,
		HandleControl::Ignore
	);

	let mut line = String::new();

	print!("test@nullex: {} $ ", *CWD.lock());
	while let Some(scancode) = scancodes.next().await {
		if let Ok(Some(key_event)) = keyboard.add_byte(scancode)
			&& let Some(key) = keyboard.process_keyevent(key_event)
		{
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
						//serial_println!("unhandled key {:?}", key);
					}
				}
				pc_keyboard::DecodedKey::Unicode(c) => {
					// backspace
					if c as u8 == 8 {
						if !line.is_empty() {
							line.pop();
							console_backspace();
						}
						continue;
					// escape: clear screen
					} else if c as u8 == 27 {
						WRITER.lock().clear_everything();
						print!("test@nullex: {} $ ", *CWD.lock());
						continue;

					// tab: handle tab completion
					} else if c as u8 == 9 {
						if line.is_empty() || line.trim().is_empty() {
							line.push_str("    ");
							print!("    ");
						} else {
							tab_completion(&mut line);
						}
						continue;
					}

					print!("{}", c);
					if c == '\n' && !line.is_empty() {
						let command_line = line.clone();
						line.clear();
						// yield to ensure that any temporary locks
						// are released before processing the command.
						yield_now().await;
						crate::task::keyboard::commands::run_command(&command_line);
						print!("test@nullex: {} $ ", *CWD.lock());
					} else {
						line.push(c);
					}
				}
			}
		}
	}
	0 // exit code
}

/// A helper function to determine whether the command should use file/directory
/// completion.
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

pub fn tab_completion(line: &mut String) {
	let parts: Vec<&str> = line.split(' ').collect();
	let part = parts[parts.len() - 1].to_string();

	let completion_type = command_supports_completion(parts[0]);
	if completion_type == CompletionType::None {
		line.push_str("    ");
		print!("    ");
		return;
	}

	fs::with_fs(|fs| {
		let files = fs.list_dir(&CWD.lock());
		let file_types = fs
			.list_dir_entry_types(&CWD.lock())
			.into_iter()
			.flatten()
			.collect::<Vec<String>>();

		if let Ok(files) = files {
			let mut matches = files
				.iter()
				.filter(|f| f.starts_with(&part))
				.collect::<Vec<_>>();

			if matches.len() == 1 {
				match completion_type {
					CompletionType::File => {
						if file_types[files.iter().position(|r| r == matches[0].as_str()).unwrap()]
							== "File"
						{
							let match_str = matches.pop().unwrap();

							// remove the part of the line that is being
							// completed.
							for _ in 0..part.len() {
								line.pop();
								console_backspace();
							}

							line.push_str(match_str);
							print!("{}", match_str);
						}
					}
					CompletionType::Directory => {
						if file_types[files.iter().position(|r| r == matches[0].as_str()).unwrap()]
							== "Directory"
						{
							let match_str = matches.pop().unwrap();

							for _ in 0..part.len() {
								line.pop();
								console_backspace();
							}

							line.push_str(match_str);
							print!("{}", match_str);
						}
					}
					CompletionType::Both => {
						if file_types[files.iter().position(|r| r == matches[0].as_str()).unwrap()]
							== "Directory" || file_types
							[files.iter().position(|r| r == matches[0].as_str()).unwrap()]
							== "File"
						{
							let match_str = matches.pop().unwrap();

							for _ in 0..part.len() {
								line.pop();
								console_backspace();
							}

							line.push_str(match_str);
							print!("{}", match_str);
						}
					}
					_ => return
				}
			}
			if matches.len() > 1 {
				println!();

				match completion_type {
					CompletionType::File => {
						for m in matches {
							if file_types[files.iter().position(|r| r == m.as_str()).unwrap()]
								== "File"
							{
								println!("{}", m);
							}
						}
					}
					CompletionType::Directory => {
						for m in matches {
							if file_types[files.iter().position(|r| r == m.as_str()).unwrap()]
								== "Directory"
							{
								println!("{}", m);
							}
						}
					}
					CompletionType::Both => {
						for m in matches {
							println!("{}", m);
						}
					}
					_ => return
				}
				print!("test@nullex: {} $ {}", *CWD.lock(), line);
			}
		}
	});
}

pub fn uparrow_completion(line: &mut String) {
	// lock the history and history index.
	let history = CMD_HISTORY.lock();
	let mut index = CMD_HISTORY_INDEX.lock();

	if history.is_empty() {
		return;
	}

	// if we're not at the oldest command move one step backward.
	if *index > 0 {
		*index -= 1;
	}

	// clear the current input from the screen.
	for _ in 0..line.len() {
		line.pop();
		console_backspace();
	}

	// get the command from history and print it.
	let cmd = &history[*index];
	print!("{}", cmd);
	line.push_str(cmd);
}

pub fn downarrow_completion(line: &mut String) {
	let history = CMD_HISTORY.lock();
	let mut index = CMD_HISTORY_INDEX.lock();

	if history.is_empty() {
		return;
	}

	// if already at the newest command clear the line and do nothing.
	if *index >= history.len() - 1 {
		// clear the current input from the screen.
		for _ in 0..line.len() {
			line.pop();
			console_backspace();
		}
		return;
	}

	// otherwise move one step forward.
	*index += 1;

	// clear the current input from the screen.
	for _ in 0..line.len() {
		line.pop();
		console_backspace();
	}

	// get the command from history and display it.
	let cmd = &history[*index];
	print!("{}", cmd);
	line.push_str(cmd);
}
