//! syscall.rs

//!
//! Syscall module for the kernel.
//!
//! Using custom system call commands as I would love this kernel to be unique
//! to me and others without resembling too much of UNIX/Linux
//!

use alloc::{string::ToString, sync::Arc};
use core::sync::atomic::{AtomicBool, Ordering};

use futures::task::AtomicWaker;

use crate::{
	arch::x86_64::user::{KERNEL_CR3, KERNEL_RETURN_ADDR, KERNEL_RETURN_RBP, KERNEL_RETURN_RSP, USER_EXIT_CODE}, fs::{self, resolve_path}, println, serial_println, task::{
		OpenFile,
		Process,
		ProcessId,
		ProcessState,
		executor::{self, CURRENT_PROCESS, EXECUTOR}
	}, utils::{elf::parse_elf, oncecell::spin::OnceCell}
};

// syscall ids

const SYS_SAY: u32 = 0;
const SYS_HALT: u32 = 1;
const SYS_SPLIT: u32 = 2;
const SYS_WAITON: u32 = 3;
const SYS_OPENF: u32 = 4;
const SYS_CLOSEF: u32 = 5;
const SYS_READF: u32 = 6;
const SYS_WRITEF: u32 = 7;
const SYS_RUN: u32 = 8;
const SYS_STOP: u32 = 9;
const SYS_NAP: u32 = 10;
const SYS_SIZEF: u32 = 11;

/// System call handler function. Called when the `syscall` or `int 0x80` instruction
/// is called.
///
/// # x86_64
/// Conforms to the conventional Linux-style syscall ABI:
/// - syscall_id in rax
/// - arg0 in rdi
/// - arg1 in rsi
/// - arg2 in rdx
/// - arg3 in r10
/// - arg4 in r8
/// - arg5 in r9
/// - return value in rax
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
			let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
			let s = unsafe { core::str::from_utf8_unchecked(bytes) };
			sys_say(s);
			0
		}
		SYS_HALT => {
			let exit_code = arg1 as i32;
			USER_EXIT_CODE.store(exit_code, Ordering::SeqCst);

			unsafe {
				core::arch::asm!(
					"mov cr3, {cr3}",
					"mov rsp, [{krsp}]",
					"mov rbp, [{krbp}]",
					"jmp [{kret}]",
					cr3  = in(reg) KERNEL_CR3,
					krsp = in(reg) core::ptr::addr_of!(KERNEL_RETURN_RSP),
					krbp = in(reg) core::ptr::addr_of!(KERNEL_RETURN_RBP),
					kret = in(reg) core::ptr::addr_of!(KERNEL_RETURN_ADDR),
					options(noreturn)
				);
			}
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
		SYS_SIZEF => {
			let fd = arg1 as u32;
			sys_sizef(fd)
		}
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
	let child_process = Process::new(child_state).expect("Process created incorrectly.");
	match executor.spawn_process(child_process) {
		Ok(()) => child_pid.get() as i32,
		Err(_) => -1, // Return error code on spawn failure
	}
}

fn sys_waiton() -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_wait: No current process guard");
			return -1;
		}
		let _process = &mut *executor::CURRENT_PROCESS_GUARD;
		0
	}
}

fn sys_say(s: &str) {
	println!("{}", s);
}

fn sys_openf(path: &str) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_openf: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		let path_r = resolve_path(path);
		let exists = fs::with_fs(|fs| fs.get_file(&path_r).is_ok());
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
				if let Ok(file) = fs.get_file(path.as_str()) {
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

fn sys_sizef(fd: u32) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_writef: No current process guard");
			return -1;
		}

		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		if let Some(open_file) = process.open_files.get(&fd) {
			let path = &open_file.path;
			fs::with_fs(|fs| {
				if !fs.exists(path) || fs.is_dir(path) { return -1isize }
				return fs.get_file(path).unwrap().content.len().try_into().unwrap()
			}).try_into().unwrap()
		} else {
			serial_println!("sys_sizef: Invalid file descriptor: {}", fd);
			-1
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
				if fs.write_file(path.as_str(), buf, false).is_ok() {
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
	let maybe_bytes = fs::with_fs(|fs| fs.get_file(path).ok().map(|f| f.content.clone()));
	let elf_bytes = match maybe_bytes {
		Some(b) => b,
		None => {
			serial_println!("sys_run: file not found: {}", path);
			return -1;
		}
	};
	let _e = parse_elf(&elf_bytes);
	0
}


fn sys_stop(pid: u64) -> i32 {
	EXECUTOR.lock().end_process(ProcessId::new(pid), -2);
	0 // placeholder: should terminate the specified process
}
