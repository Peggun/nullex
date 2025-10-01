pub mod ata;
pub mod ramfs;

use alloc::{
	string::{String, ToString},
	vec::Vec
};

use spin::Mutex;

use crate::fs::ramfs::FileSystem;

pub static FS: Mutex<Option<FileSystem>> = Mutex::new(None);

pub fn init_fs(fs: FileSystem) {
	*FS.lock() = Some(fs);
}

pub fn with_fs<R>(f: impl FnOnce(&mut FileSystem) -> R) -> R {
	let mut fs_lock = FS.lock();
	let fs_ref = fs_lock.as_mut().expect("Filesystem must be initialized");

	// release VGA lock before FS operations
	unsafe { crate::vga_buffer::WRITER.force_unlock() };
	let result = f(fs_ref);
	crate::vga_buffer::WRITER.lock();

	result
}

/// Helper function to resolve a file path relative to the current working
/// directory.
pub fn resolve_path(path: &str) -> String {
	use crate::task::keyboard::scancode::CWD;

	let mut cwd = CWD.lock().clone();
	let mut result = if path.starts_with('/') {
		String::new()
	} else {
		cwd.push('/');
		cwd
	};
	result.push_str(path);
	normalize_path(&result)
}

pub fn normalize_path(path: &str) -> String {
	let parts: Vec<&str> = path
		.split('/')
		.filter(|&p| !p.is_empty() && p != ".")
		.collect();
	let mut stack = Vec::new();
	for part in parts {
		if part == ".." {
			if !stack.is_empty() {
				stack.pop();
			}
		} else {
			stack.push(part);
		}
	}
	if stack.is_empty() {
		"/".to_string()
	} else {
		format!("/{}/", stack.join("/"))
	}
}
