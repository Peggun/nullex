// syscall.rs

/*
Syscall module for the kernel.
*/

use alloc::{string::ToString, sync::Arc};
use core::sync::atomic::AtomicBool;

use crate::{
	fs,
	println,
	serial_println,
	task::{
		OpenFile,
		Process,
		ProcessState,
		executor::{self, CURRENT_PROCESS, EXECUTOR}
	}
};

// System call IDs
pub const SYS_PRINT: u32 = 1;
pub const SYS_EXIT: u32 = 2;
pub const SYS_FORK: u32 = 3;
pub const SYS_WAIT: u32 = 4;
pub const SYS_OPEN: u32 = 5;
pub const SYS_CLOSE: u32 = 6;
pub const SYS_READ: u32 = 7;
pub const SYS_WRITE: u32 = 8;
pub const SYS_EXEC: u32 = 9;
pub const SYS_KILL: u32 = 10;

// System call handler function
pub fn syscall(syscall_id: u32, arg1: u64, arg2: u64, arg3: u64, _arg4: u64, _arg5: u64) -> i32 {
	match syscall_id {
		SYS_PRINT => {
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::str::from_raw_parts(ptr, len) };
			sys_print(s);
			0
		}
		SYS_EXIT => {
			let exit_code = arg1 as i32;
			sys_exit(exit_code);
		}
		SYS_FORK => sys_fork(),
		SYS_WAIT => sys_wait(),
		SYS_OPEN => {
			let path_ptr = arg1 as *const u8;
			let path_len = arg2 as usize;
			let path = unsafe { core::str::from_raw_parts(path_ptr, path_len) };
			sys_open(path)
		}
		SYS_CLOSE => {
			let fd = arg1 as u32;
			sys_close(fd)
		}
		SYS_READ => {
			let fd = arg1 as u32;
			let buf_ptr = arg2 as *mut u8;
			let len = arg3 as usize;
			sys_read(fd, buf_ptr, len)
		}
		SYS_WRITE => {
			let fd = arg1 as u32;
			let buf_ptr = arg2 as *const u8;
			let len = arg3 as usize;
			sys_write(fd, buf_ptr, len)
		}
		SYS_EXEC => {
			let path_ptr = arg1 as *const u8;
			let path_len = arg2 as usize;
			let path = unsafe { core::str::from_raw_parts(path_ptr, path_len) };
			sys_exec(path)
		}
		SYS_KILL => {
			let pid = arg1 as u64;
			sys_kill(pid)
		}
		_ => {
			serial_println!("Invalid syscall ID: {}", syscall_id);
			-1 // Error code for unhandled syscall
		}
	}
}

// --- Syscall implementations ---

// Process management

pub fn sys_fork() -> i32 {
	serial_println!("sys_fork called");
	let current_state = {
		let locked = CURRENT_PROCESS.lock();
		locked
			.as_ref()
			.expect("No current process during sys_fork")
			.clone()
	};
	let future_fn_clone = current_state.future_fn.clone();
	let mut executor = EXECUTOR.lock();
	let child_pid = executor.create_pid();
	let child_state = Arc::new(ProcessState {
		id: child_pid,
		is_child: true,
		future_fn: future_fn_clone,
		queued: AtomicBool::new(false)
	});
	let child_process = Process::new(child_state);
	executor.spawn_process(child_process);
	child_pid.get() as i32
}

pub fn sys_wait() -> i32 {
	// Placeholder: should wait for a child process to complete
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_wait: No current process guard");
			return -1;
		}
		let _process = &mut *executor::CURRENT_PROCESS_GUARD;
		// TODO: Implement waiting for a child process
		0 // Placeholder return value
	}
}

pub fn sys_print(s: &str) {
	println!("{}", s);
}

pub fn sys_exit(exit_code: i32) -> ! {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_exit: No current process guard");
		} else {
			let _process = &mut *executor::CURRENT_PROCESS_GUARD;
			println!("Process exiting with code: {}", exit_code);
		}
		panic!("sys_exit called - process should terminate (simplified behavior)")
	}
}

// File operations

pub fn sys_open(path: &str) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_open: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		let exists = fs::with_fs(|fs| fs.get_file(path).is_ok());
		if !exists {
			serial_println!("sys_open: File not found: {}", path);
			return -1;
		}
		let fd = process.next_fd;
		process.open_files.insert(fd, OpenFile {
			path: path.to_string(),
			offset: 0
		});
		process.next_fd += 1;
		fd as i32
	}
}

pub fn sys_close(fd: u32) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_close: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		if process.open_files.remove(&fd).is_some() {
			0 // Success
		} else {
			serial_println!("sys_close: Invalid file descriptor: {}", fd);
			-1 // Error: invalid fd
		}
	}
}

pub fn sys_read(fd: u32, buf_ptr: *mut u8, len: usize) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_read: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		if let Some(open_file) = process.open_files.get_mut(&fd) {
			let path = &open_file.path;
			let offset = open_file.offset;
			fs::with_fs(|fs| {
				if let Ok(file) = fs.get_file(path) {
					let bytes_to_read =
						core::cmp::min(len, file.content.len().saturating_sub(offset));
					if bytes_to_read > 0 {
						let buf = core::slice::from_raw_parts_mut(buf_ptr, bytes_to_read);
						buf.copy_from_slice(&file.content[offset..offset + bytes_to_read]);
						open_file.offset += bytes_to_read;
						bytes_to_read as i32
					} else {
						0 // End of file
					}
				} else {
					serial_println!("sys_read: File not found: {}", path);
					-1 // Error: file not found
				}
			})
		} else {
			serial_println!("sys_read: Invalid file descriptor: {}", fd);
			-1 // Error: invalid fd
		}
	}
}

pub fn sys_write(fd: u32, buf_ptr: *const u8, len: usize) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_write: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		if let Some(open_file) = process.open_files.get(&fd) {
			let path = &open_file.path;
			let buf = core::slice::from_raw_parts(buf_ptr, len);
			let result = fs::with_fs(|fs| {
				if fs.write_file(path, buf).is_ok() {
					len as i32 // Number of bytes written
				} else {
					serial_println!("sys_write: Write failed: {}", path);
					-1 // Error: write failed (e.g., permission denied)
				}
			});
			result
		} else {
			serial_println!("sys_write: Invalid file descriptor: {}", fd);
			-1 // Error: invalid fd
		}
	}
}

// Placeholder implementations

pub fn sys_exec(path: &str) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_exec: No current process guard");
			return -1;
		}
		let _process = &mut *executor::CURRENT_PROCESS_GUARD;
		serial_println!("sys_exec: Executing {} (not implemented)", path);
		0 // Placeholder: should replace process image
	}
}

pub fn sys_kill(pid: u64) -> i32 {
	// Does not need current process state, only executor access
	serial_println!("sys_kill: Killing PID {} (not implemented)", pid);
	0 // Placeholder: should terminate the specified process
}
