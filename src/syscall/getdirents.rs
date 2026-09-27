use alloc::vec::Vec;

use crate::{
	error::ERR_BAD_FD,
	fs::{self, SCDirectoryEntryInfo},
	memory::user::UserMutSlice,
	serial_println,
	task::{executor, files::FileBackend}
};

pub fn sys_getdirents(fd: u32, out: &UserMutSlice<SCDirectoryEntryInfo>) -> i32 {
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
		let open_files = &mut process.open_files;

		let file_desc = match open_files.get_mut(&fd) {
			Some(f) => f,
			None => return ERR_BAD_FD // EBADF
		};

		serial_println!("{:#?}", file_desc);

		if !matches!(file_desc.backend, FileBackend::Directory { .. }) {
			serial_println!("sys_getdirents: fd is not a directory.");
			return -1;
		}

		// Build the entries in a kernel-side buffer first, then copy the
		// whole thing across the user/kernel boundary in one checked write.
		let mut kernel_entries: Vec<SCDirectoryEntryInfo> = Vec::with_capacity(out.len);

		{
			fs::with_fs(|fs| {
				if let Some(path) = file_desc.path() {
					if let Ok(dir) = fs.get_dir(path) {
						for (name, entry) in dir.entries.iter() {
							if kernel_entries.len() >= out.len {
								break;
							}

							match SCDirectoryEntryInfo::try_from((name.as_str(), entry)) {
								Ok(info) => {
									kernel_entries.push(info);
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

		match out.copy_from_kernel(&kernel_entries) {
			Ok(written) => written as i32,
			Err(_) => {
				serial_println!("sys_getdirents: failed to copy entries to user buffer");
				ERR_BAD_FD
			}
		}
	}
}
