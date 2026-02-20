//! syscall.rs

//!
//! Syscall module for the kernel.
//!
//! Using custom system call commands as I would love this kernel to be unique
//! to me and others without resembling too much of UNIX/Linux
//!

use alloc::{string::ToString, sync::Arc};
use core::sync::atomic::AtomicBool;

use futures::task::AtomicWaker;

use crate::{
	fs,
	println,
	serial_println,
	task::{
		OpenFile,
		Process,
		ProcessId,
		ProcessState,
		executor::{self, CURRENT_PROCESS, EXECUTOR}
	},
	utils::oncecell::spin::OnceCell
};

// syscall ids

const SYS_SAY: u32 = 1;
const SYS_HALT: u32 = 2;
const SYS_SPLIT: u32 = 3;
const SYS_WAITON: u32 = 4;
const SYS_OPENF: u32 = 5;
const SYS_CLOSEF: u32 = 6;
const SYS_READF: u32 = 7;
const SYS_WRITEF: u32 = 8;
const SYS_RUN: u32 = 9;
const SYS_STOP: u32 = 10;
const SYS_NAP: u32 = 11;

/// System call handler function. Called when the `syscall` or `int 0x80` instruction
/// is called. 
/// 
/// # x86 (`int 0x80`)
/// - syscall_id in eax
/// - arg1 in ebx
/// - arg2 in ecx
/// - arg3 in edx
/// - arg4 in esi 
/// - arg5 in edi
/// - arg6 in ebp (not supported)
/// 
/// # x86_64 (`syscall`)
/// - syscall_id in erax
/// - arg1 in rbx
/// - arg2 in rcx
/// - arg3 in rdx
/// - arg4 in rsi 
/// - arg5 in rdi
/// - arg6 in rbp (not supported)
/// 
/// # Safety
/// - Make sure valid arguments
pub unsafe fn syscall(
	syscall_id: u32,
	arg1: u64,
	arg2: u64,
	arg3: u64,
	_arg4: u64,
	_arg5: u64
) -> i32 {
	match syscall_id {
		SYS_SAY => {
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::str::from_raw_parts(ptr, len) };
			sys_say(s);
			0
		}
		SYS_HALT => {
			let exit_code = arg1 as i32;
			sys_halt(exit_code);
		}
		SYS_SPLIT => sys_split(),
		SYS_WAITON => sys_waiton(),
		SYS_OPENF => {
			let path_ptr = arg1 as *const u8;
			let path_len = arg2 as usize;
			let path = unsafe { core::str::from_raw_parts(path_ptr, path_len) };
			sys_openf(path)
		}
		SYS_CLOSEF => {
			let fd = arg1 as u32;
			sys_closef(fd)
		}
		SYS_READF => {
			let fd = arg1 as u32;
			let buf_ptr = arg2 as *mut u8;
			let len = arg3 as usize;
			unsafe { sys_readf(fd, buf_ptr, len) }
		}
		SYS_WRITEF => {
			let fd = arg1 as u32;
			let buf_ptr = arg2 as *const u8;
			let len = arg3 as usize;
			unsafe { sys_writef(fd, buf_ptr, len) }
		}
		SYS_RUN => {
			let path_ptr = arg1 as *const u8;
			let path_len = arg2 as usize;
			let path = unsafe { core::str::from_raw_parts(path_ptr, path_len) };
			sys_run(path)
		}
		SYS_STOP => sys_stop(arg1),
		SYS_NAP => {
			serial_println!("i go nap nap now. sleep is a) broken, and b) unsafe :(");
			0
		},
		_ => {
			serial_println!("Invalid syscall ID: {}", syscall_id);
			-1 // error code for unhandled syscall
		}
	}
}

fn sys_split() -> i32 {
	serial_println!("sys_split called");
	let current_state = {
		let locked = CURRENT_PROCESS.lock();
		locked
			.as_ref()
			.expect("No current process during sys_split")
			.clone()
	};
	let future_fn_clone = current_state.future_fn.clone();
	let mut executor = EXECUTOR.lock();
	let child_pid = executor.create_pid();
	let child_state = Arc::new(ProcessState {
		id: child_pid,
		is_child: true,
		future_fn: future_fn_clone,
		queued: AtomicBool::new(false),
		scancode_queue: OnceCell::uninit(),
		waker: AtomicWaker::new()
	});
	let child_process = Process::new(child_state);
	executor.spawn_process(child_process);
	child_pid.get() as i32
}

fn sys_waiton() -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_wait: No current process guard");
			return -1;
		}
		let _process = &mut *executor::CURRENT_PROCESS_GUARD;
		todo!();
		// implement waiting for a child process
		//0
	}
}

fn sys_say(s: &str) {
	println!("{}", s);
}

fn sys_halt(exit_code: i32) -> ! {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_halt: No current process guard");
		} else {
			let _process = &mut *executor::CURRENT_PROCESS_GUARD;
			println!("Process exiting with code: {}", exit_code);
		}
		panic!("sys_halt called - process should terminate (simplified behavior)")
	}
}

fn sys_openf(path: &str) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_openf: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		let exists = fs::with_fs(|fs| fs.get_file(path).is_ok());
		if !exists {
			serial_println!("sys_openf: File not found: {}", path);
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

fn sys_closef(fd: u32) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_closef: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		if process.open_files.remove(&fd).is_some() {
			0 // success
		} else {
			serial_println!("sys_closef: Invalid file descriptor: {}", fd);
			-1 // invalid fd
		}
	}
}

/// # Safety
/// `buf_ptr` needs to be a valid pointer or else undefined behaviour
unsafe fn sys_readf(fd: u32, buf_ptr: *mut u8, len: usize) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_readf: No current process guard");
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
						0 // eof
					}
				} else {
					serial_println!("sys_readf: File not found: {}", path);
					-1 // file not found
				}
			})
		} else {
			serial_println!("sys_readf: Invalid file descriptor: {}", fd);
			-1 // invalid fd
		}
	}
}

/// # Safety
/// `buf_ptr` needs to be a valid pointer or else undefined behaviour
unsafe fn sys_writef(fd: u32, buf_ptr: *const u8, len: usize) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_writef: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		if let Some(open_file) = process.open_files.get(&fd) {
			let path = &open_file.path;
			let buf = core::slice::from_raw_parts(buf_ptr, len);
			fs::with_fs(|fs| {
				if fs.write_file(path, buf, false).is_ok() {
					len as i32 // number of bytes written
				} else {
					serial_println!("sys_writef: Write failed: {}", path);
					-1 // write failed
				}
			})
		} else {
			serial_println!("sys_writef: Invalid file descriptor: {}", fd);
			-1 // invalid fd
		}
	}
}

fn sys_run(path: &str) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_exec: No current process guard");
			return -1;
		}
		let _process = &mut *executor::CURRENT_PROCESS_GUARD;
		serial_println!("sys_exec: Executing {} (not implemented)", path);
		0 // placeholder: should replace process image
	}
}

fn sys_stop(pid: u64) -> i32 {
	EXECUTOR.lock().end_process(ProcessId::new(pid), -2);
	0 // placeholder: should terminate the specified process
}
