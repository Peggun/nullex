/*
	A super simple text editor for the kernel.
	This code is completely outrageous, and i will refactor this.
	I'm pushing this code for feedback on where to improve on
	various platforms.
	So see some big changes soon hopefully
*/

use alloc::{
	boxed::Box,
	string::{String, ToString},
	sync::Arc
};
use core::{future::Future, pin::Pin};

use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;
use pc_keyboard::{HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1, layouts};
use spin::mutex::Mutex;

use crate::{
	fs::{self, resolve_path},
	print,
	println,
	serial_println,
	task::{
		keyboard::{
			commands::clear, KEYBOARD_BACKSPACE, KEYBOARD_ENTER, KEYBOARD_ESCAPE, KEYBOARD_RAW_KEYS, KEYBOARD_TAB
		}, ProcessState
	},
	utils::process::spawn_process,
	vga_buffer::{Buffer, BufferEntry, WRITER}
};

lazy_static! {
	pub static ref PREV_BUFFER: Mutex<Buffer> = Mutex::new(Buffer::blank());
	pub static ref PREV_CUR_POS: Mutex<(usize, usize)> = Mutex::new((0, 0));
}

pub static mut CTRL_PRESSED: bool = false;
pub static mut ASKING_TO_SAVE: bool = false;
pub static mut MADE_CHANGES: bool = false;

pub fn nedit_app(args: &[&str]) {

	println!("code is depreciated. a kernel doesnt actually need these. when a package manager becomes available for nullex, i will happily add this to the repo.");
	return;

	if args.is_empty() {
		println!("nedit: missing file operand.");
		return;
	}

	let path = resolve_path(args[0]);

	let exists = fs::with_fs(|fs| fs.exists(&path));

	if !exists {
		println!("nedit: {}: No such file ", path);
		return
	}
	// main function for nedit.
	// doesnt spawn as a child process
	// cause like nedit.exe for eg runs as its own process.
	let _ = spawn_process(
		move |state| {
			let path = path.clone();
			Box::pin(nedit_main(state, path)) as Pin<Box<dyn Future<Output = i32>>>
		},
		false
	);
}

pub async fn nedit_main(state: Arc<ProcessState>, path: String) -> i32 {
	let fc: Option<String> = fs::with_fs(|fs| match fs.read_file(&path) {
		Ok(content) => {
			let s = String::from_utf8_lossy_owned(content.to_vec());
			Some(s)
		}
		Err(_) => None
	});

	if fc.is_none() {
		return 1;
	}

	// we can unwrap here because we have checked if its none.
	let mut fc = fc.unwrap();

	unsafe {
		// setup scancode queue and keyboard inputs
		// like the keyboard driver
		state
			.scancode_queue
			.try_init_once(|| ArrayQueue::new(100))
			.expect("Scancode stream should only be called once. NEdit.");

		let mut keyboard = Keyboard::new(
			ScancodeSet1::new(),
			layouts::Us104Key,
			HandleControl::Ignore
		);

		let snapshot = {
			let mut writer = WRITER.lock();
			writer.copy_vga_buffer()
		};

		let cur_pos_snapshot = {
			let mut writer = WRITER.lock();
			writer.copy_cursor_position()
		};

		{
			let mut prev = PREV_BUFFER.lock();
			let mut prev_cur_pos = PREV_CUR_POS.lock();
			*prev = snapshot;
			*prev_cur_pos = cur_pos_snapshot;
		}

		{
			let mut writer = WRITER.lock();
			writer.clear_everything();
		}

		print!("{}", fc);

		// helper functions: convert between (row,col) and byte index in fc
		fn byte_index_from_row_col(s: &str, target_row: usize, target_col: usize) -> usize {
			let mut row = 0usize;
			let mut col = 0usize;
			if target_row == 0 && target_col == 0 {
				return 0;
			}
			for (i, ch) in s.char_indices() {
				if row == target_row && col == target_col {
					return i;
				}
				if ch == '\n' {
					row += 1;
					col = 0;
					// If we just moved to the next row and the target is that new row at col 0
					if row == target_row && target_col == 0 {
						return i + ch.len_utf8();
					}
				} else {
					col += 1;
				}
			}
			// If we walked the whole string, return end of string
			s.len()
		}

		fn row_col_from_byte_index(s: &str, byte_idx: usize) -> (usize, usize) {
			let mut row = 0usize;
			let mut col = 0usize;
			let mut reached = false;
			for (i, ch) in s.char_indices() {
				if i >= byte_idx {
					reached = true;
					break;
				}
				if ch == '\n' {
					row += 1;
					col = 0;
				} else {
					col += 1;
				}
			}
			if !reached && byte_idx >= s.len() {
				// cursor at end of file
				// if the file ends with a newline, cursor should be at start of next line
				return (row, col);
			}
			(row, col)
		}

		fn prev_char_start(s: &str, byte_idx: usize) -> Option<usize> {
			if byte_idx == 0 {
				return None;
			}
			let mut prev = None;
			for (i, _) in s.char_indices() {
				if i >= byte_idx {
					break;
				}
				prev = Some(i);
			}
			prev
		}

		// return length (in columns/chars) of a given row
		fn line_length(s: &str, target_row: usize) -> usize {
			let mut row = 0usize;
			let mut col = 0usize;
			for (_i, ch) in s.char_indices() {
				if row == target_row {
					if ch == '\n' {
						return col;
					}
					col += 1;
				} else if ch == '\n' {
					row += 1;
				}
			}
			if row == target_row {
				return col; // last line
			}
			0
		}

		// return number of rows (0-based last row index is rows-1)
		fn total_rows(s: &str) -> usize {
			if s.is_empty() {
				return 0;
			}
			let mut rows = 1usize;
			for ch in s.chars() {
				if ch == '\n' {
					rows += 1;
				}
			}
			rows
		}

		// clamp column to valid range for given row
		fn clamp_col_for_row(s: &str, row: usize, col: usize) -> usize {
			let len = line_length(s, row);
			if col > len { len } else { col }
		}

		// main app loop
		loop {
			// print keypress for now
			// damn i gotta fix this.
			while let Some(queue) = state.scancode_queue.get().iter().next() {
				if let Some(c) = queue.pop()
					&& let Ok(Some(key_event)) = keyboard.add_byte(c)
				{
					// handle control state
					if key_event.code == KeyCode::LControl
						|| key_event.code == KeyCode::RControl
						|| key_event.code == KeyCode::RControl2
					{
						if key_event.state == KeyState::Down {
							CTRL_PRESSED = true;
						} else if key_event.state == KeyState::Up {
							CTRL_PRESSED = false;
						}
					}

					// arrow key movement: ensure we clamp columns and handle line-ends
					if key_event.state == KeyState::Down
						&& (key_event.code == KeyCode::ArrowDown
							|| key_event.code == KeyCode::ArrowLeft
							|| key_event.code == KeyCode::ArrowRight
							|| key_event.code == KeyCode::ArrowUp)
					{
						let mut writer = WRITER.lock();
						let mut cur_row = writer.current_row;
						let mut cur_col = writer.column_position;
						let rows = total_rows(&fc);

						match key_event.code {
							KeyCode::ArrowDown => {
								if cur_row + 1 < rows {
									cur_row += 1;
									// clamp column to length of target line
									cur_col = clamp_col_for_row(&fc, cur_row, cur_col);
								}
							}
							KeyCode::ArrowUp => {
								if cur_row > 0 {
									cur_row -= 1;
									cur_col = clamp_col_for_row(&fc, cur_row, cur_col);
								}
							}
							KeyCode::ArrowLeft => {
								if cur_col > 0 {
									cur_col -= 1;
								} else if cur_row > 0 {
									// move to end of previous line
									cur_row -= 1;
									cur_col = line_length(&fc, cur_row);
								}
							}
							KeyCode::ArrowRight => {
								let line_len = line_length(&fc, cur_row);
								if cur_col < line_len {
									cur_col += 1;
								} else if cur_row + 1 < rows {
									// move to start of next line
									cur_row += 1;
									cur_col = 0;
								}
							}
							_ => {}
						}

						writer.current_row = cur_row;
						writer.column_position = cur_col;
						writer.update_cursor();
						drop(writer);
					}

					if let Some(key) = keyboard.process_keyevent(key_event.clone())
						&& let pc_keyboard::DecodedKey::Unicode(ch) = key
					{
						// handle special control-like keys by comparing to known
						// constants
						if ch as u8 == KEYBOARD_BACKSPACE {
							// delete char before cursor
							let (cur_row, cur_col) = {
								let writer = WRITER.lock();
								(writer.current_row, writer.column_position)
							};
							let idx = byte_index_from_row_col(&fc, cur_row, cur_col);
							if idx == 0 {
								// nothing to delete
								continue;
							}
							if let Some(prev_idx) = prev_char_start(&fc, idx) {
								fc.replace_range(prev_idx..idx, "");
								// redraw entire buffer and restore cursor
								let (new_r, new_c) = row_col_from_byte_index(&fc, prev_idx);
								{
									let mut writer = WRITER.lock();
									writer.clear_everything();
									// ensure printing starts at top-left
									writer.current_row = 0;
									writer.column_position = 0;
								}
								// release lock before printing to avoid deadlock
								print!("{}", fc);
								{
									let mut writer = WRITER.lock();
									writer.current_row = new_r;
									// clamp just in case
									writer.column_position = clamp_col_for_row(&fc, new_r, new_c);
									writer.update_cursor();
								}
								if !MADE_CHANGES && !KEYBOARD_RAW_KEYS.contains(&(ch as u8)) {
									MADE_CHANGES = true;
								}
							}
							continue;
						} else if ch as u8 == KEYBOARD_TAB {
							// insert 4 spaces at cursor
							let (cur_row, cur_col) = {
								let writer = WRITER.lock();
								(writer.current_row, writer.column_position)
							};
							let idx = byte_index_from_row_col(&fc, cur_row, cur_col);
							fc.insert_str(idx, "    ");
							let (new_r, new_c) = row_col_from_byte_index(&fc, idx + 4); // moved 4 columns
							{
								let mut writer = WRITER.lock();
								writer.clear_everything();
								writer.current_row = 0;
								writer.column_position = 0;
							}
							print!("{}", fc);
							{
								let mut writer = WRITER.lock();
								writer.current_row = new_r;
								writer.column_position = clamp_col_for_row(&fc, new_r, new_c);
								writer.update_cursor();
							}
							if !MADE_CHANGES {
								MADE_CHANGES = true;
							}
							continue;
						} else if ch as u8 == KEYBOARD_ENTER {
							// insert newline at cursor position and move cursor to
							// beginning of next line
							let (cur_row, cur_col) = {
								let writer = WRITER.lock();
								(writer.current_row, writer.column_position)
							};
							let idx = byte_index_from_row_col(&fc, cur_row, cur_col);
							fc.insert(idx, '\n');
							// redraw and set cursor to next line col 0
							{
								let mut writer = WRITER.lock();
								writer.clear_everything();
								writer.current_row = 0;
								writer.column_position = 0;
							}
							print!("{}", fc);
							{
								let mut writer = WRITER.lock();
								// compute new cursor position based on byte index
								// after the inserted newline
								let (new_r, new_c) =
									row_col_from_byte_index(&fc, idx + '\n'.len_utf8());
								writer.current_row = new_r;
								writer.column_position = clamp_col_for_row(&fc, new_r, new_c);
								writer.update_cursor();
							}
							if !MADE_CHANGES {
								MADE_CHANGES = true;
							}
							continue;
						}

						if CTRL_PRESSED {
							if ch.to_lowercase().to_string() == "q" {
								clear(&[""]);
								println!(
									"Would you like to save your changes?\n     Y     N     Esc (go back)"
								);

								if MADE_CHANGES {
									ASKING_TO_SAVE = true;
									continue;
								}

								// return 0, for quitting the app
								return quit()
							}

							print!("^{}", ch.to_uppercase());
							continue;
						}

						if ASKING_TO_SAVE && ch.to_lowercase().to_string() == "y" {
							serial_println!("y was pressed. saving...");

							fs::with_fs(|fs| fs.write_file(&path, fc.as_bytes(), true)).unwrap();

							return quit();
						} else if ASKING_TO_SAVE && ch.to_lowercase().to_string() == "n" {
							serial_println!("n was pressed. exiting...");
							ASKING_TO_SAVE = false;
							return quit()
						} else if ASKING_TO_SAVE && ch as u8 == KEYBOARD_ESCAPE {
							clear(&[""]);
							println!("{}", fc);
							ASKING_TO_SAVE = false;
							continue
						}

						let (cur_row, cur_col) = {
							let writer = WRITER.lock();
							(writer.current_row, writer.column_position)
						};
						let idx = byte_index_from_row_col(&fc, cur_row, cur_col);
						fc.insert(idx, ch);
						let (new_r, new_c) = row_col_from_byte_index(&fc, idx + ch.len_utf8());
						{
							let mut writer = WRITER.lock();
							writer.clear_everything();
							writer.current_row = 0;
							writer.column_position = 0;
						}
						print!("{}", fc);
						{
							let mut writer = WRITER.lock();
							writer.current_row = new_r;
							writer.column_position = clamp_col_for_row(&fc, new_r, new_c);
							writer.update_cursor();
						}

						if !MADE_CHANGES && !KEYBOARD_RAW_KEYS.contains(&(ch as u8)) {
							MADE_CHANGES = true;
						}
					}
				}
			}
		}
	}
}

pub fn quit() -> i32 {
	let mut writer = WRITER.lock();
	let prev_b = PREV_BUFFER.lock();
	let prev_cur_pos = PREV_CUR_POS.lock();
	writer.clear_everything();
	writer.restore_vga_buffer(&prev_b);

	writer.current_row = prev_cur_pos.0;
	writer.column_position = prev_cur_pos.1;

	writer.update_cursor();

	// remove the locks
	drop(writer);
	drop(prev_b);
	drop(prev_cur_pos);

	0
}
