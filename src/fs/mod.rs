//!
//! fs/mod.rs
//!
//! Top level filesystem module declaration.

#[allow(missing_docs)]
pub mod ata;
pub mod ramfs;

use alloc::{
	string::{String, ToString},
	vec::Vec
};

use crate::{
	drivers::keyboard::scancode::CWD,
	fs::ramfs::{Entry, FileSystem, FsError, Permission},
	utils::mutex::SpinMutex
};

// TODO: maybe lazy_static!
/// Current `FileSystem` in use.
pub static FS: SpinMutex<Option<FileSystem>> = SpinMutex::new(None);

/// Initialises the kernel's `FileSystem`
pub fn init_fs(fs: FileSystem) {
	*FS.lock() = Some(fs);
}

/// Use the current `FileSystem` to perform an action.
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

fn normalize_path(path: &str) -> String {
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

// ---------- SYSCALLS ---------- //
// because we are unable to pass HashMap<> and Box<> through syscalls i have to
// create new unanimous types which are like a bridge between Rust and C
// they will all be prefixed with SC-
const NULLEX_NAME_MAX: usize = 256;

pub const OPEND_NONE: u64 = 0;
pub const OPEND_RESOLVE: u64 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SCPermission {
	pub read: u8,
	pub write: u8,
	pub execute: u8
}

impl From<Permission> for SCPermission {
	fn from(value: Permission) -> Self {
		Self {
			read: value.read as u8,
			write: value.write as u8,
			execute: value.execute as u8
		}
	}
}

#[repr(u32)]
pub enum SCFsErrorCode {
	Ok = 0,
	EntryNotFound = 1,
	NotADirectory = 2,
	NotAFile = 3,
	PermissionDenied = 4,
	AlreadyExists = 5,
	InvalidPath = 6,
	DirectoryNotEmpty = 7
}

impl From<FsError> for SCFsErrorCode {
	fn from(value: FsError) -> Self {
		match value {
			FsError::EntryNotFound => SCFsErrorCode::EntryNotFound,
			FsError::NotADirectory => SCFsErrorCode::NotADirectory,
			FsError::NotAFile => SCFsErrorCode::NotAFile,
			FsError::PermissionDenied => SCFsErrorCode::PermissionDenied,
			FsError::AlreadyExists => SCFsErrorCode::AlreadyExists,
			FsError::InvalidPath => SCFsErrorCode::InvalidPath,
			FsError::DirectoryNotEmpty => SCFsErrorCode::DirectoryNotEmpty,
			FsError::Generic => SCFsErrorCode::Ok
		}
	}
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SCEntryKind {
	File = 0,
	Directory = 1
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SCDirectoryEntryInfo {
	pub kind: SCEntryKind,
	pub permission: SCPermission,
	pub size: u64,
	pub name_len: u32,
	pub name: [u8; NULLEX_NAME_MAX]
}

impl<'a> TryFrom<(&'a str, &'a Entry)> for SCDirectoryEntryInfo {
	type Error = FsError;

	fn try_from(value: (&'a str, &'a Entry)) -> Result<Self, Self::Error> {
		let (name, entry) = value;

		let mut out = SCDirectoryEntryInfo {
			kind: SCEntryKind::File,
			permission: SCPermission {
				read: 0,
				write: 0,
				execute: 0
			},
			size: 0,
			name_len: 0,
			name: [0; NULLEX_NAME_MAX]
		};

		let bytes = name.as_bytes();
		let take = bytes.len().min(NULLEX_NAME_MAX);
		out.name[..take].copy_from_slice(&bytes[..take]);
		out.name_len = take as u32;

		match entry {
			Entry::File(file) => {
				out.kind = SCEntryKind::File;
				out.permission = file.permission.into();
				out.size = file.len() as u64;
			}
			Entry::Directory(dir) => {
				out.kind = SCEntryKind::Directory;
				out.permission = dir.permission.into();
				out.size = dir.entries.len() as u64;
			}
		}

		Ok(out)
	}
}
