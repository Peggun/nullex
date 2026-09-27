use crate::{serial_println, task::executor};

pub fn sys_closed(fd: u32) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_closed: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		if process.open_files.remove(&fd).is_some() {
			0
		} else {
			serial_println!("sys_closed: Invalid file descriptor: {}", fd);
			-1
		}
	}
}
