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
pub const SYS_SLEEP: u32 = 11;

/// A trait that defines the low-level syscall interface.
pub trait Syscalls {
	/// The generic syscall function.
	///
	/// # Safety
	/// This function is unsafe because it performs raw syscalls.
	unsafe fn syscall(&self, id: u32, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64)
	-> i32;

	/// Convenience method for printing a string.
	fn sys_print(&self, s: &str) -> i32 {
		let ptr = s.as_ptr() as u64;
		let len = s.len() as u64;
		unsafe { self.syscall(SYS_PRINT, ptr, len, 0, 0, 0) }
	}

	fn sys_exit(&self, exit_code: i32) -> i32 {
		unsafe { self.syscall(SYS_EXIT, exit_code as u64, 0, 0, 0, 0) }
	}

	fn sys_fork(&self) -> i32 {
		unsafe { self.syscall(SYS_FORK, 0, 0, 0, 0, 0) }
	}

	fn sys_wait(&self) -> i32 {
		unsafe { self.syscall(SYS_WAIT, 0, 0, 0, 0, 0) }
	}

	fn sys_open(&self, path: &str) -> i32 {
		let path_ptr = path.as_ptr() as u64;
		let path_len = path.len() as u64;
		unsafe { self.syscall(SYS_OPEN, path_ptr, path_len, 0, 0, 0) }
	}

	fn sys_close(&self, fd: u32) -> i32 {
		unsafe { self.syscall(SYS_CLOSE, fd as u64, 0, 0, 0, 0) }
	}

	fn sys_read(&self, fd: u32, buf_ptr: *mut u8, len: usize) -> i32 {
		unsafe { self.syscall(SYS_READ, fd as u64, buf_ptr as u64, len as u64, 0, 0) }
	}

	fn sys_exec(&self, path: &str) -> i32 {
		let path_ptr = path.as_ptr() as u64;
		let path_len = path.len() as u64;
		unsafe { self.syscall(SYS_EXEC, path_ptr, path_len, 0, 0, 0) }
	}

	fn sys_kill(&self, pid: u64) -> i32 {
		unsafe { self.syscall(SYS_KILL, pid, 0, 0, 0, 0) }
	}
}
