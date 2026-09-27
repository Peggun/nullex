use crate::{
	error::{ERR_INVALID, ERR_NO_ENT},
	fs,
	memory::user::UserSlice,
	serial_println
};

pub fn sys_rmdir(path: &UserSlice<u8>) -> i32 {
	let path_bytes = unsafe { core::slice::from_raw_parts(path.ptr, path.len) };

	let path_str = match core::str::from_utf8(path_bytes) {
		Ok(path) => path,
		Err(_) => return ERR_INVALID
	};

	serial_println!("sys_rmdir: path = {}", path_str);

	fs::with_fs(|fs| match fs.remove(path_str, true, false) {
		Ok(_) => 0,
		Err(_) => ERR_NO_ENT
	})
}
