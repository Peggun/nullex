use alloc::vec::Vec;
use core::sync::atomic::Ordering;

use crate::{
	fs,
	io::keyboard::line_editor::{LINE_READY, PROGRAM_WAITING, STDIN_BUFFER},
	memory::user::UserMutSlice,
	serial_println,
	task::{executor, files::FileBackend}
};

pub fn sys_readf(fd: u32, dst: &UserMutSlice<u8>) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_readf: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;

		if fd != 0 && fd != 1 && fd != 2 {
			// regular file read
			if let Some(open_file) = process.open_files.get_mut(&fd) {
				if let FileBackend::DiskFile {
					ref path,
					ref mut offset
				} = open_file.backend
				{
					fs::with_fs(|fs| {
						if let Ok(file_len) = fs.file_len(path.as_str()) {
							let bytes_to_read =
								core::cmp::min(dst.len, file_len.saturating_sub(*offset));
							if bytes_to_read > 0 {
								let mut kernel_buf = vec![0u8; bytes_to_read];
								let bytes_read = fs
									.read_file_at(path.as_str(), *offset, &mut kernel_buf)
									.unwrap_or(0);
								*offset += bytes_read;

								match dst.copy_from_kernel(&kernel_buf[..bytes_read]) {
									Ok(written) => written as i32,
									Err(_) => {
										serial_println!("sys_readf: failed to copy to user buffer");
										-1
									}
								}
							} else {
								0 // eof
							}
						} else {
							serial_println!("sys_readf: File not found: {}", path);
							-1
						}
					})
				} else {
					-1
				}
			} else {
				serial_println!("sys_readf: Invalid file descriptor: {}", fd);
				-1
			}
		} else if fd == 0 {
			// stdin
			{
				let mut stdin = STDIN_BUFFER.lock();
				stdin.clear();
			}
			LINE_READY.store(false, Ordering::SeqCst);

			PROGRAM_WAITING.store(true, Ordering::SeqCst);

			serial_println!("sys_readf: waiting for stdin");

			// sleep until the line is ready
			while !LINE_READY.load(Ordering::SeqCst) {
				x86_64::instructions::interrupts::enable_and_hlt();
			}

			serial_println!("sys_readf: got line, draining buffer");

			PROGRAM_WAITING.store(false, Ordering::SeqCst);

			let mut stdin = STDIN_BUFFER.lock();
			let bytes_to_read = core::cmp::min(dst.len, stdin.len());
			let mut kernel_buf = Vec::with_capacity(bytes_to_read);
			for _ in 0..bytes_to_read {
				if let Some(byte) = stdin.pop_front() {
					kernel_buf.push(byte);
				}
			}
			if stdin.is_empty() {
				LINE_READY.store(false, Ordering::SeqCst);
			}
			stdin.clear();

			let result = match dst.copy_from_kernel(&kernel_buf) {
				Ok(written) => written as i32,
				Err(_) => {
					serial_println!("sys_readf: failed to copy stdin to user buffer");
					-1
				}
			};

			serial_println!("sys_readf: returning {} bytes", result);
			result
		} else {
			serial_println!("sys_readf: fd {} is not readable", fd);
			-1
		}
	}
}
