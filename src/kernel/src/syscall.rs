// syscall.rs

/*
Syscall module for the kernel.
*/

use alloc::{string::ToString, sync::Arc};
use core::{
	arch::{asm, naked_asm},
	sync::atomic::AtomicBool
};

use orchestrator::syscall_interface::*;
use x86_64::registers::model_specific::{Efer, EferFlags, Msr};

use crate::{
	apic::apic::sleep,
	fs,
	println,
	serial_println,
	task::{
		OpenFile,
		Process,
		ProcessId,
		ProcessState,
		executor::{self, CURRENT_PROCESS, EXECUTOR}
	}
};

pub struct KernelSyscalls;

impl Syscalls for KernelSyscalls {
	unsafe fn syscall(
		&self,
		id: u32,
		arg1: u64,
		arg2: u64,
		arg3: u64,
		arg4: u64,
		arg5: u64
	) -> i32 {
		let result: i32;
		unsafe {
			asm!(
				"syscall",
				in("rax") id,
				in("rdi") arg1,
				in("rsi") arg2,
				in("rdx") arg3,
				in("r10") arg4,
				in("r8")  arg5,
				lateout("rax") result,
				clobber_abi("sysv64"),
			);
		}
		result
	}
}

#[naked]
#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler() {
	unsafe {
		naked_asm!(
		"
            push rcx  // Save user RIP
            push r11  // Save user RFLAGS
            // Stack: [..., user RIP, user RFLAGS]

            call {rust_handler}  // Call Rust handler

            pop r11   // Restore user RFLAGS
            pop rcx   // Restore user RIP
            sysretq   // Return to user mode
            ",
		rust_handler = sym rust_syscall_handler,
		options()
		);
	}
}

pub fn rust_syscall_handler() {
	let syscall_id: u32;
	let arg1: u64;
	let arg2: u64;
	let arg3: u64;
	let arg4: u64;
	let arg5: u64;

	unsafe {
		asm!(
			"mov {0:e}, eax", // Syscall ID
			"mov {1}, rdi",   // Arg 1
			"mov {2}, rsi",   // Arg 2
			"mov {3}, rdx",   // Arg 3
			"mov {4}, r10",   // Arg 4
			"mov {5}, r8",    // Arg 5
			out(reg) syscall_id,
			out(reg) arg1,
			out(reg) arg2,
			out(reg) arg3,
			out(reg) arg4,
			out(reg) arg5,
		);
	}

	let result = crate::syscall::syscall(syscall_id, arg1, arg2, arg3, arg4, arg5);

	unsafe {
		asm!(
		"mov rax, {0}",  // Return value in RAX
		in(reg) result as u64,
		);
	}
}

/// Initialises the syscalls for the Nullex Kernel
pub fn init_syscalls() {
	unsafe {
		// Enable the syscall instruction (set SCE bit in EFER)
		let mut efer = Efer::read();
		efer |= EferFlags::SYSTEM_CALL_EXTENSIONS;
		Efer::write(efer);

		// Set IA32_LSTAR to the syscall handler address
		unsafe extern "C" {
			fn syscall_handler();
		}
		Msr::new(0xC0000082).write(syscall_handler as u64); // IA32_LSTAR

		// Set IA32_STAR for segment selectors
		let kernel_cs = 0x08; // Kernel code segment (index 1)
		let _user_cs = 0x1B; // User code segment (index 3 | RPL 3)
		let user_ss = 0x23; // User data segment (index 4 | RPL 3)
		let star = ((user_ss as u64) << 48) | ((kernel_cs as u64) << 32);
		Msr::new(0xC0000081).write(star); // IA32_STAR

		// Set IA32_FMASK to clear the Interrupt Flag (IF) on syscall entry
		Msr::new(0xC0000084).write(1 << 9); // IA32_FMASK
	}
}

// System call handler function
pub fn syscall(syscall_id: u32, arg1: u64, arg2: u64, arg3: u64, _arg4: u64, _arg5: u64) -> i32 {
	match syscall_id {
		SYS_PRINT => {
			let ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let s = unsafe { core::str::from_raw_parts(ptr, len) };
			sys_print(s)
		}
		SYS_EXIT => {
			let exit_code = arg1 as i32;
			sys_exit(exit_code);
			//0
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
			serial_println!("Invalid syscall ID: {}", syscall_id); // Replace with your logging function
			-1 // Error code for unhandled syscall
		}
	}
}

pub unsafe fn invoke_syscall(id: u32, arg1: u64, arg2: u64) {
	unsafe {
		asm!(
			"int 0x80",
			in("rax") id,
			in("rdi") arg1,
			in("rsi") arg2,
		);
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

pub fn sys_print(s: &str) -> i32 {
	println!("{}", s);
	0
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
	EXECUTOR.lock().end_process(ProcessId::new(pid), -2);
	serial_println!("Killed: {}", pid);
	0 // Placeholder: should terminate the specified process
}

pub async fn sys_sleep(duration: u32) -> i32 {
	unsafe {
		let _ = sleep(duration).await;
	};
	0
}
