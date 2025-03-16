use core::arch::asm;

/*
Register Orders based on these

https://wiki.osdev.org/System_V_ABI
https://refspecs.linuxbase.org/elf/x86_64-abi-0.99.pdf

*/
use orchestrator::syscall_interface::*;

pub struct UserspaceSyscalls<'a> {
	pub iface: &'a dyn Syscalls
}

// System call IDs (must match kernel's syscall.rs)

/// Generic syscall function with up to 5 arguments.
/// # Arguments
/// * `id: u32` - The syscall ID.
/// * `arg1: u64` - The first argument.
/// * `arg2: u64` - The second argument.
/// * `arg3: u64` - The third argument.
/// * `arg4: u64` - The fourth argument.
/// * `arg5: u64` - The fifth argument.
///
/// # Returns
/// * The return value of the syscall.
pub unsafe fn syscall(id: u32, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i32 {
	unsafe {
		let result: i32;
		asm!(
			"syscall",
			in("rax") id,     // Syscall ID in RAX
			in("rdi") arg1,   // First argument
			in("rsi") arg2,   // Second argument
			in("rdx") arg3,   // Third argument
			in("r10") arg4,   // Fourth argument
			in("r8") arg5,    // Fifth argument
			lateout("rax") result, // Return value
			clobber_abi("sysv64"), // Specify calling convention
		);
		result
	}
}

// All SYSCALLS return either Ok(()) or a Err(KernelError)

/// Prints a string by invoking the kernel's SYS_PRINT system call.
/// # Returns
/// 0 - Success
/// -1 - Error
// pub fn sys_print(s: &str) -> i32 {
// 	let ptr = s.as_ptr() as u64; // Pointer to the string
// 	let len = s.len() as u64; // Length of the string
// 	kernel::syscall::syscall(SYS_PRINT, ptr, len, 0, 0, 0) // Call syscall with
// ID=1, ptr, len, and dummy 0 }

// pub fn sys_exit(exit_code: i32) -> i32 {
// 	kernel::syscall::syscall(SYS_EXIT, exit_code.try_into().unwrap(), 0, 0, 0,
// 0) }

// pub fn sys_fork() -> i32 {
// 	kernel::syscall::syscall(SYS_FORK, 0, 0, 0, 0, 0)
// }

// pub fn sys_wait() -> i32 {
// 	kernel::syscall::syscall(SYS_WAIT, 0, 0, 0, 0, 0)
// }

// pub fn sys_open(path: &str) -> i32 {
// 	let path_ptr = path.as_ptr() as u64;
// 	let path_len = path.len() as u64;
// 	kernel::syscall::syscall(SYS_OPEN, path_ptr, path_len, 0, 0, 0)
// }

// pub fn sys_close(fd: u32) -> i32 {
// 	kernel::syscall::syscall(SYS_CLOSE, fd as u64, 0, 0, 0, 0)
// }

// pub fn sys_read(fd: u32, buf_ptr: *mut u8, len: usize) -> i32 {
// 	kernel::syscall::syscall(SYS_READ, fd as u64, buf_ptr as u64, len as u64, 0,
// 0) }

// pub fn sys_write(fd: u32, buf_ptr: *mut u8, len: usize) -> i32 {
// 	kernel::syscall::syscall(SYS_WRITE, fd as u64, buf_ptr as u64, len as u64,
// 0, 0) }

// pub fn sys_exec(path: &str) -> i32 {
// 	let path_ptr = path.as_ptr() as u64;
// 	let path_len = path.len() as u64;
// 	kernel::syscall::syscall(SYS_EXEC, path_ptr, path_len, 0, 0, 0)
// }

// pub fn sys_kill(pid: u64) -> i32 {
// 	kernel::syscall::syscall(SYS_KILL, pid, 0, 0, 0, 0)
// }

impl<'a> UserspaceSyscalls<'a> {
	pub fn sys_print(&self, s: &str) -> i32 {
		self.iface.sys_print(s)
	}

	pub fn sys_exit(&self, exit_code: i32) -> i32 {
		self.iface.sys_exit(exit_code)
	}

	pub fn sys_fork(&self) -> i32 {
		self.iface.sys_fork()
	}

	pub fn sys_wait(&self) -> i32 {
		self.iface.sys_wait()
	}

	pub fn sys_open(&self, path: &str) -> i32 {
		self.iface.sys_open(path)
	}

	pub fn sys_close(&self, fd: u32) -> i32 {
		self.iface.sys_close(fd)
	}

	pub fn sys_read(&self, fd: u32, buf_ptr: *mut u8, len: usize) -> i32 {
		self.iface.sys_read(fd, buf_ptr, len)
	}

	pub fn sys_exec(&self, path: &str) -> i32 {
		self.iface.sys_exec(path)
	}

	pub fn sys_kill(&self, pid: u64) -> i32 {
		self.iface.sys_kill(pid)
	}
}
