use crate::{
	error::ERR_BAD_FD,
	fs,
	memory::user::UserSlice,
	serial_println,
	task::{executor, files::FileBackend}
};

pub fn sys_writef(fd: u32, src: &UserSlice<u8>) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_writef: No current process guard");
			return -1;
		}

		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		if let Some(open_file) = process.open_files.get_mut(&fd) {
            if let FileBackend::DiskFile {
                ref path,
                ref mut offset
            } = open_file.backend
            {
                let mut kernel_buf = vec![0u8; src.len];
                if let Err(_) = src.copy_to_kernel(&mut kernel_buf) {
                    serial_println!("sys_writef: failed to copy from user buffer");
                    return ERR_BAD_FD;
                }
      		
                fs::with_fs(|fs| {
                    if let Ok(len) = fs.write_file_at(path.as_str(), &kernel_buf, *offset) {
                        *offset += len;
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
            ERR_BAD_FD
		}
	}
}
