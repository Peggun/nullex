use alloc::{
	string::{String, ToString},
	vec::Vec
};

use crate::{common::ffi::c_strlen, serial_println};

pub unsafe fn argv_to_vec(
	argc: usize,
	argv: *const *const u8
) -> Result<Vec<String>, core::str::Utf8Error> {
	let argc = argc;
	let mut args = Vec::with_capacity(argc);

	if argv.is_null() {
		serial_println!("argv_to_vec: NULL");
		return Ok(Vec::new());
	}

	unsafe {
		for i in 1..argc {
			let ptr = *argv.add(i);
			if ptr.is_null() {
				break;
			}
			let len = c_strlen(ptr);
			let arg = core::str::from_utf8(core::slice::from_raw_parts(ptr, len))?.to_string();
			args.push(arg);
		}
	}

	for arg in &args {
		serial_println!("arg: {}", arg);
	}

	Ok(args)
}
