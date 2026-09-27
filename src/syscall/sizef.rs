use crate::{
	error::{ERR_IS_DIR, ERR_NO_ENT},
	fs,
	serial_println,
	task::{executor, files::FileBackend}
};

pub fn sys_sizef(fd: u32) -> i32 {
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
					if !fs.exists(path) {
						return ERR_NO_ENT
					}
					if fs.is_dir(path) {
						return ERR_IS_DIR
					}
					let len = fs.file_len(path).expect("Unable to read file length.");
					return len as i32;
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
