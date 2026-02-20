//!
//! completion.rs
//! 
//! Keyboard command completion logic for the kernel.
//! 

use alloc::{
	string::{String, ToString},
	vec::Vec
};

use crate::{
	drivers::keyboard::scancode::CWD,
	fs,
	print,
	println,
	task::keyboard::commands::{CMD_HISTORY, CMD_HISTORY_INDEX},
	vga_buffer::console_backspace
};

#[derive(Debug, PartialEq)]
enum CompletionType {
	File,
	Directory,
	Both,
	None
}

fn command_supports_completion(command: &str) -> CompletionType {
	match command {
		"cd" | "ls" | "rmdir" => CompletionType::Directory,
		"cat" | "write" | "rm" => CompletionType::File,
		"clear" | "help" | "exit" | "echo" | "mkdir" | "progs" | "kill" | "touch" => {
			CompletionType::None
		}
		_ => CompletionType::Both
	}
}

/// Complete the command with the use of the `TAB` key
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

/// Uparrow completion. Goes through the command history.
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

/// Downarrow completion. Goes through the command history
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
