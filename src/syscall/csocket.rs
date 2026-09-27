use crate::{
	serial_println,
	task::{
		executor,
		files::{FileBackend, OpenFile}
	}
};

pub fn sys_csocket() -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_csocket: no current process guard");
			return -1;
		}

		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		let fd = process.next_fd;
		process.open_files.insert(fd, OpenFile {
			backend: FileBackend::Socket {
				connection: None
			}
		});
		process.next_fd += 1;
		fd as i32
	}
}
