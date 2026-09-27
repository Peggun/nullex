use crate::{
	fs::{self, ramfs::Permission, resolve_path},
	serial_println,
	syscall::O_CREAT,
	task::{
		executor,
		files::{FileBackend, OpenFile}
	}
};

pub fn sys_openf(path: &str, flags: u32) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_openf: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		let path_r = resolve_path(path.trim_end_matches("\n"));

		let exists = fs::with_fs(|fs| fs.exists(&path_r));

		if !exists {
			if (flags & O_CREAT) == 0 {
				serial_println!("sys_openf: File not found and O_CREAT not set: {}", path);
				return -1;
			}

			let create_result = fs::with_fs(|fs| fs.create_file(&path_r, Permission::readwrite()));
			if create_result.is_err() {
				serial_println!("sys_openf: Failed to create file: {}", path);
				return -1;
			}
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
