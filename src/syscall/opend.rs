use alloc::string::ToString;

use crate::{
	error::{ERR_NO_ENT, ERR_NOT_DIR},
	fs::{self, ramfs::Permission},
	serial_println,
	syscall::O_CREAT,
	task::{
		executor,
		files::{FileBackend, OpenFile}
	}
};

pub fn sys_opend(path: &str, flags: u32) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_opend: No current process guard");
			return -1;
		}
		let process = &mut *executor::CURRENT_PROCESS_GUARD;

		let exists = fs::with_fs(|fs| fs.get_dir(&path).is_ok());
		if !exists {
			if flags & O_CREAT != 0 {
				let create_result = fs::with_fs(|fs| fs.create_dir(&path, Permission::readwrite()));
				if create_result.is_err() {
					serial_println!("sys_opend: Failed to create directory: {}", path);
					return -1;
				}
			} else {
				serial_println!("sys_opend: File not found: {}", path);
				return ERR_NO_ENT;
			}
		}
		let is_dir = fs::with_fs(|fs| fs.is_dir(&path));
		if !is_dir {
			serial_println!("sys_opend: Not a directory: {}", path);
			return ERR_NOT_DIR;
		}

		let fd = process.next_fd;
		process.open_files.insert(fd, OpenFile {
			backend: FileBackend::Directory {
				path: path.to_string()
			}
		});
		process.next_fd += 1;
		fd as i32
	}
}
