//!
//! line_editor.rs
//! 
//! Keypress printing handler for the kernel.
//! 

use alloc::string::String;

use futures::StreamExt;

use crate::{
	drivers::keyboard::{
		layouts,
		ps2::Keyboard,
		queue::ScancodeStream,
		scancode::{CWD, KeyCode, ScancodeSet1}
	}, io::keyboard::{
		completion::{downarrow_completion, tab_completion, uparrow_completion},
		decode::{DecodedKey, HandleControl}
	}, print, print_colours, task::yield_now, vga_buffer::{WRITER, console_backspace}
};

/// The async function that reads scancodes and processes keypresses.
pub async fn print_keypresses() -> i32 {
	let mut scancodes = ScancodeStream::new();

	let mut keyboard = Keyboard::new(
		ScancodeSet1::new(),
		layouts::us104::Us104Key,
		HandleControl::Ignore
	);

	let mut line = String::new();

	//print!("test@nullex: {} $ ", *CWD.lock());
	print_colours!(
		("test", Color::Green),
		(&format!("@nullex: {} $ ", *CWD.lock()), Color::White)
	);
	while let Some(scancode) = scancodes.next().await {
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
					} else {
						//serial_println!("unhandled key {:?}", key);
					}
				}
				DecodedKey::Unicode(c) => {
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
						print_colours!(
							("test", Color::Green),
							(&format!("@nullex: {} $ ", *CWD.lock()), Color::White)
						);
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
						print_colours!(
							("test", Color::Green),
							(&format!("@nullex: {} $ ", *CWD.lock()), Color::White)
						);
					} else {
						line.push(c);
					}
				}
			}
		}
	}
	0 // exit code
}
