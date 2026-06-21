//! syscall.rs

//!
//! Syscall module for the kernel.
//!
//! Using custom system call commands as I would love this kernel to be unique
//! to me and others without resembling too much of UNIX/Linux

use alloc::{
	string::{String, ToString},
	sync::Arc,
	vec::Vec
};
use core::sync::atomic::{AtomicBool, Ordering};

use futures::task::AtomicWaker;

#[cfg(target_arch = "x86_64")]
use crate:: {
	arch::x86_64::user::{
		KERNEL_CR3,
		KERNEL_RETURN_ADDR,
		KERNEL_RETURN_RBP,
		KERNEL_RETURN_RSP,
		USER_EXIT_CODE
	},
};

use crate::{
	error::{ERR_BAD_FD, NullexError::OutOfMemory},
	fs::{
		self,
		OPEND_RESOLVE,
		SCDirectoryEntryInfo,
		SCEntryKind::File,
		ramfs::Entry,
		resolve_path
	},
	io::keyboard::line_editor::{LINE_READY, PROGRAM_WAITING, STDIN_BUFFER},
	print,
	println,
	serial,
	serial_println,
	task::{
		FileBackend,
		OpenFile,
		Process,
		ProcessId,
		ProcessState,
		executor::{self, CURRENT_PROCESS, CURRENT_PROCESS_GUARD, EXECUTOR}
	},
	utils::{elf::parse_elf, oncecell::spin::OnceCell}
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
const SYS_OPEND: u32 = 8;
const SYS_RUN: u32 = 9;
const SYS_STOP: u32 = 10;
const SYS_NAP: u32 = 11;
const SYS_SIZEF: u32 = 12;
const SYS_CSOCKET: u32 = 13;
const SYS_CONNSOCK: u32 = 14;
const SYS_SEND: u32 = 15;
const SYS_RECV: u32 = 16;
const SYS_CLOSESOCK: u32 = 17;
const SYS_GETDIRENTS: u32 = 18;

/// System call handler function. Called when the `syscall` or `int 0x80`
/// instruction is called.
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
#[cfg(target_arch = "x86_64")]
pub unsafe fn syscall(
	syscall_id: u32,
	arg0: u64,
	arg1: u64,
	arg2: u64,
	_arg3: u64,
	_arg4: u64,
	_arg5: u64,
) -> i32 {
	match syscall_id {
		SYS_SAY => {
			let ptr = arg0 as *const u8;
			let len = arg1 as usize;
			let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
			let s = unsafe { core::str::from_utf8_unchecked(bytes) };
			sys_say(s);
			0
		}
		SYS_HALT => {
			let exit_code = arg0 as i32;
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
			let path_ptr = arg0 as *const u8;
			let path_len = arg1 as usize;
			let path = unsafe { core::str::from_raw_parts(path_ptr, path_len) };
			sys_openf(path)
		}
		SYS_CLOSEF => {
			let fd = arg0 as u32;
			sys_closef(fd)
		}
		SYS_READF => {
			let fd = arg0 as u32;
			let buf_ptr = arg1 as *mut u8;
			let len = arg2 as usize;
			unsafe { sys_readf(fd, buf_ptr, len) }
		}
		SYS_WRITEF => {
			let fd = arg0 as u32;
			let buf_ptr = arg1 as *const u8;
			let len = arg2 as usize;
			unsafe { sys_writef(fd, buf_ptr, len) }
		}
		SYS_OPEND => {
			let path_ptr = arg0 as *const u8;
			let path_len = arg1 as usize;
			let path = unsafe { core::str::from_raw_parts(path_ptr, path_len) };
			sys_opend(path, arg2)
		}
		SYS_RUN => {
			let path_ptr = arg0 as *const u8;
			let path_len = arg1 as usize;
			let path = unsafe { core::str::from_raw_parts(path_ptr, path_len) };
			sys_run(path)
		}
		SYS_STOP => sys_stop(arg0),
		SYS_NAP => {
			serial_println!("i go nap nap now. sleep is a) broken, and b) unsafe :(");
			0
		}
		SYS_SIZEF => {
			let fd = arg0 as u32;
			sys_sizef(fd)
		}
		SYS_GETDIRENTS => {
			let fd = arg0 as u32;
			let out_ptr = arg1 as *mut SCDirectoryEntryInfo;
			let out_cap = arg2 as usize;
			sys_getdirents(fd, out_ptr, out_cap)
		}
		_ => {
			serial_println!("Invalid syscall ID: {}", syscall_id);
			-1
		}
	}
}

/// AArch64 System call handler function. Called when the `svc #0``
/// instruction is called.
///
/// # x86_64
/// Conforms to the conventional Linux-style syscall ABI:
/// - syscall_id in x8
/// - arg0 in x0
/// - arg1 in x1
/// - arg2 in x2
/// - arg3 in x3
/// - arg4 in x4
/// - arg5 in x5
/// - return value in x0
///
/// # Safety
/// - Make sure valid arguments
#[cfg(target_arch = "aarch64")]
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
			let ptr = arg0 as *const u8;
			let len = arg1 as usize;
			let bytes = unsafe { core::slice::from_raw_parts(ptr, len) };
			let s = unsafe { core::str::from_utf8_unchecked(bytes) };
			sys_say(s);
			0
		}
		SYS_HALT => {
			let exit_code = arg0 as i32;
			USER_EXIT_CODE.store(exit_code, Ordering::SeqCst);

			unsafe {
				core::arch::asm!(
					"mov sp, {krsp}",
					"mov x29, {krbp}",
					"br {kret}",
					krsp = in(reg) KERNEL_RETURN_RSP,
					krbp = in(reg) KERNEL_RETURN_RBP,
					kret = in(reg) KERNEL_RETURN_ADDR,
					options(noreturn)
				);
			}

		}
		SYS_SPLIT => sys_split(),
		SYS_WAITON => sys_waiton(),
		SYS_OPENF => {
			let path_ptr = arg0 as *const u8;
			let path_len = arg1 as usize;
			let path = unsafe { core::str::from_raw_parts(path_ptr, path_len) };
			sys_openf(path)
		}
		SYS_CLOSEF => {
			let fd = arg0 as u32;
			sys_closef(fd)
		}
		SYS_READF => {
			let fd = arg0 as u32;
			let buf_ptr = arg1 as *mut u8;
			let len = arg2 as usize;
			unsafe { sys_readf(fd, buf_ptr, len) }
		}
		SYS_WRITEF => {
			let fd = arg0 as u32;
			let buf_ptr = arg1 as *const u8;
			let len = arg2 as usize;
			unsafe { sys_writef(fd, buf_ptr, len) }
		}
		SYS_OPEND => {
			let path_ptr = arg0 as *const u8;
			let path_len = arg1 as usize;
			let path = unsafe { core::str::from_raw_parts(path_ptr, path_len) };
			sys_opend(path, arg2)
		}
		SYS_RUN => {
			let path_ptr = arg0 as *const u8;
			let path_len = arg1 as usize;
			let path = unsafe { core::str::from_raw_parts(path_ptr, path_len) };
			sys_run(path)
		}
		SYS_STOP => sys_stop(arg0),
		SYS_NAP => {
			serial_println!("i go nap nap now. sleep is a) broken, and b) unsafe :(");
			0
		}
		SYS_SIZEF => {
			let fd = arg0 as u32;
			sys_sizef(fd)
		}
		SYS_GETDIRENTS => {
			let fd = arg0 as u32;
			let out_ptr = arg1 as *mut SCDirectoryEntryInfo;
			let out_cap = arg2 as usize;
			sys_getdirents(fd, out_ptr, out_cap)
		}
		_ => {
			serial_println!("Invalid syscall ID: {}", syscall_id);
			-1
		}
	}
}

fn sys_getdirents(fd: u32, out_ptr: *mut SCDirectoryEntryInfo, out_cap: usize) -> i32 {
	unsafe {
		if fd == 0 || fd == 1 || fd == 2 {
			serial_println!("sys_getdirents: fd {} is not valid.", fd);
			return ERR_BAD_FD;
		}

		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_getdirents: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		let mut open_files = &mut process.open_files;

		let file_desc = match open_files.get_mut(&fd) {
			Some(f) => f,
			None => return ERR_BAD_FD // EBADF
		};

		serial_println!("{:#?}", file_desc);

		if !matches!(file_desc.backend, FileBackend::Directory { .. }) {
			serial_println!("sys_getdirents: fd is not a directory.");
			return -1;
		}

		let mut written = 0usize;
		let out = core::slice::from_raw_parts_mut(out_ptr, out_cap);

		{
			fs::with_fs(|fs| {
				if let Some(path) = file_desc.path() {
					if let Ok(dir) = fs.get_dir(path) {
						for (name, entry) in dir.entries.iter() {
							if written >= out.len() {
								break;
							}

							match SCDirectoryEntryInfo::try_from((name.as_str(), entry)) {
								Ok(info) => {
									out[written] = info;
									written += 1;
								}
								Err(_) => {
									return;
								}
							}
						}
					}
				}
			})
		}

		written as i32
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
		Err(_) => -1
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
	print!("{}", s);
}

fn sys_openf(path: &str) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_openf: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		let path_r = resolve_path(path.trim_end_matches("\n"));
		let exists = fs::with_fs(|fs| fs.exists(&path_r));
		if !exists {
			serial_println!("sys_openf: File not found: {}", path);
			return -1;
		}
		let fd = process.next_fd;
		process.open_files.insert(fd, OpenFile {
			backend: FileBackend::DiskFile {
				path: path_r,
				offset: 0
			}
		});
		process.next_fd += 1;
		fd as i32
	}
}

fn sys_opend(path: &str, arg: u64) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_opend: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		let path_r = if arg == OPEND_RESOLVE {
			resolve_path(path.trim_end_matches("\n"))
		} else {
			path.to_string()
		};

		let exists = fs::with_fs(|fs| fs.get_dir(&path_r).is_ok());
		if !exists {
			serial_println!("sys_opend: File not found: {}", path);
			return -1;
		}
		let fd = process.next_fd;
		process.open_files.insert(fd, OpenFile {
			backend: FileBackend::Directory {
				path: path_r
			}
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
			0
		} else {
			serial_println!("sys_closef: Invalid file descriptor: {}", fd);
			-1
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
								core::cmp::min(len, file_len.saturating_sub(*offset));
							if bytes_to_read > 0 {
								let buf = core::slice::from_raw_parts_mut(buf_ptr, bytes_to_read);
								let bytes_read =
									fs.read_file_at(path.as_str(), *offset, buf).unwrap_or(0);
								*offset += bytes_read;
								bytes_read as i32
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
			let bytes_to_read = core::cmp::min(len, stdin.len());
			for i in 0..bytes_to_read {
				if let Some(byte) = stdin.pop_front() {
					*buf_ptr.add(i) = byte;
				}
			}
			if stdin.is_empty() {
				LINE_READY.store(false, Ordering::SeqCst);
			}

			serial_println!("sys_readf: returning {} bytes", bytes_to_read);
			stdin.clear();

			bytes_to_read as i32
		} else {
			serial_println!("sys_readf: fd {} is not readable", fd);
			-1
		}
	}
}

fn sys_sizef(fd: u32) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_sizef: No current process guard");
			return -1;
		}

		if fd == 0 || fd == 1 || fd == 2 {
			serial_println!("sys_sizef: fd is either stdin, stdout, or stderr.");
			return -1;
		}

		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		if let Some(open_file) = process.open_files.get(&fd) {
			if let FileBackend::DiskFile {
				ref path,
				offset: _
			} = open_file.backend
			{
				fs::with_fs(|fs| {
					if !fs.exists(path) || fs.is_dir(path) {
						return -1isize
					}
					let len = fs.file_len(path);
					return 1000;
				})
				.try_into()
				.unwrap()
			} else {
				unreachable!()
			}
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
			if let FileBackend::DiskFile {
				ref path,
				offset: _
			} = open_file.backend
			{
				let buf = core::slice::from_raw_parts(buf_ptr, len);
				fs::with_fs(|fs| {
					if fs.write_file(path.as_str(), buf, false).is_ok() {
						len as i32
					} else {
						serial_println!("sys_writef: Write failed: {}", path);
						-1
					}
				})
			} else {
				0
			}
		} else {
			serial_println!("sys_writef: Invalid file descriptor: {}", fd);
			-1
		}
	}
}

fn sys_run(path: &str) -> i32 {
	let maybe_bytes = fs::with_fs(|fs| fs.read_file_to_vec(path).ok());
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
	0
}

fn sys_csocket() -> i32 {
	0
}
