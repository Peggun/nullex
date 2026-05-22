//!
//! line_editor.rs
//!
//! Keypress printing handler for the kernel.

use alloc::{collections::vec_deque::VecDeque, string::String};
use core::sync::atomic::{AtomicBool, Ordering};

use futures::StreamExt;

use crate::{
	drivers::keyboard::{
		layouts,
		ps2::Keyboard,
		queue::ScancodeStream,
		scancode::{CWD, KeyCode, ScancodeSet1}
	},
	io::keyboard::{
		completion::{downarrow_completion, tab_completion, uparrow_completion},
		decode::{DecodedKey, HandleControl}
	},
	lazy_static,
	print,
	print_colours,
	task::yield_now,
	utils::mutex::SpinMutex,
	vga_buffer::{WRITER, console_backspace}
};

lazy_static! {
	pub static ref STDIN_BUFFER: SpinMutex<VecDeque<u8>> = SpinMutex::new(VecDeque::new());
	pub static ref LINE_READY: AtomicBool = AtomicBool::new(false);
	pub static ref PROGRAM_WAITING: AtomicBool = AtomicBool::new(false);
	static ref STDIN_KEYBOARD: SpinMutex<Keyboard<layouts::us104::Us104Key, ScancodeSet1>> =
		SpinMutex::new(Keyboard::new(
			ScancodeSet1::new(),
			layouts::us104::Us104Key,
			HandleControl::Ignore
		));
}

/// Called from add_scancode (the keyboard interrupt path) on every scancode.
///
/// When PROGRAM_WAITING is true the async executor is frozen inside
/// enter_user_process, so the normal print_keypresses loop can never run.
/// By decoding and feeding stdin here — directly in the interrupt handler —
/// we can satisfy the sys_readf spin without the executor running at all.
pub fn process_scancode_for_stdin(scancode: u8) {
	if !PROGRAM_WAITING.load(Ordering::SeqCst) {
		return;
	}

	let mut keyboard = STDIN_KEYBOARD.lock();
	if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
		if let Some(decoded) = keyboard.process_keyevent(key_event) {
			if let DecodedKey::Unicode(c) = decoded {
				handle_stdin_char(c);
			}
		}
	}
}

/// Feed one decoded character into STDIN_BUFFER, echo it, and signal on
/// newline.
fn handle_stdin_char(c: char) {
	if c == '\n' {
		print!("{}", c);
		let mut stdin = STDIN_BUFFER.lock();
		stdin.push_back(b'\n');
		LINE_READY.store(true, Ordering::SeqCst);
	} else if c as u8 == 8 {
		// Backspace
		let mut stdin = STDIN_BUFFER.lock();
		if stdin.pop_back().is_some() {
			console_backspace();
		}
	} else {
		print!("{}", c);
		let mut stdin = STDIN_BUFFER.lock();
		stdin.push_back(c as u8);
	}
}

/// The async function that reads scancodes and processes keypresses.
pub async fn print_keypresses() -> i32 {
	let mut scancodes = ScancodeStream::new();

	let mut keyboard = Keyboard::new(
		ScancodeSet1::new(),
		layouts::us104::Us104Key,
		HandleControl::Ignore
	);

	let mut line = String::new();

	print_colours!(
		("test", Color::Green),
		(&format!("@nullex: {} $ ", *CWD.lock()), Color::White)
	);

	while let Some(scancode) = scancodes.next().await {
		if PROGRAM_WAITING.load(Ordering::SeqCst) {
			continue;
		}

		if let Ok(Some(key_event)) = keyboard.add_byte(scancode)
			&& let Some(key) = keyboard.process_keyevent(key_event)
		{
			match key {
				DecodedKey::RawKey(key) => {
					if key == KeyCode::LControl
						|| key == KeyCode::RControl
						|| key == KeyCode::RControl2
					{
						print_colours!(
							("^C", Color::White),
							("test", Color::Green),
							(&format!("@nullex: {} $ ", *CWD.lock()), Color::White)
						);
						line.clear();
					} else if key == KeyCode::ArrowUp {
						uparrow_completion(&mut line);
					} else if key == KeyCode::ArrowDown {
						downarrow_completion(&mut line);
					}
				}
				DecodedKey::Unicode(c) => {
					if c as u8 == 8 {
						if !line.is_empty() {
							line.pop();
							console_backspace();
						}
						continue;
					} else if c as u8 == 27 {
						WRITER.lock().clear_everything();
						print_colours!(
							("test", Color::Green),
							(&format!("@nullex: {} $ ", *CWD.lock()), Color::White)
						);
						line.clear();
						continue;
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

						{
							let mut stdin = STDIN_BUFFER.lock();
							stdin.clear();
						}
						LINE_READY.store(false, Ordering::SeqCst);

						yield_now().await;
						crate::task::keyboard::commands::run_command(&command_line);
						print_colours!(
							("test", Color::Green),
							(&format!("@nullex: {} $ ", *CWD.lock()), Color::White)
						);
					} else if c != '\n' {
						line.push(c);
					}
				}
			}
		}
	}
	0
}
