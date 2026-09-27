//!
//! ramfs.rs
//!
//! RamFS implementation for the kernel.

use alloc::{
	boxed::Box,
	string::{String, ToString},
	vec::Vec
};
use core::{fmt, str};

use hashbrown::HashMap;

use crate::fs::{SCFsErrorCode, SCPermission, init_fs};

include!(concat!(env!("OUT_DIR"), "/userspace_registry.rs"));

#[derive(Debug, Clone, Copy, PartialEq)]
/// Permission Levels for file access.
pub struct Permission {
	/// Can read from a file
	pub read: bool,
	/// Can write to a file
	pub write: bool,
	/// Can execute a file.
	pub execute: bool
}

impl Permission {
	/// All permissions for a file.
	pub fn all() -> Self {
		Self {
			read: true,
			write: true,
			execute: true
		}
	}

	/// Read-only permissions for a file.
	pub fn read() -> Self {
		Self {
			read: true,
			write: false,
			execute: false
		}
	}

	pub fn write() -> Self {
		Self {
			read: false,
			write: true,
			execute: false
		}
	}

	/// No permissions for a file.
	pub fn none() -> Self {
		Self {
			read: false,
			write: false,
			execute: false
		}
	}

	/// Execute-only permissions for a file.
	pub fn execute() -> Self {
		Self {
			read: false,
			write: false,
			execute: true
		}
	}

	/// Read-write permissions for a file.
	pub fn readwrite() -> Self {
		Self {
			read: true,
			write: true,
			execute: false
		}
	}
}

impl From<SCPermission> for Permission {
	fn from(value: SCPermission) -> Self {
		Self {
			read: value.read != 0,
			write: value.write != 0,
			execute: value.execute != 0
		}
	}
}

#[derive(Debug)]
/// Structure representing a file in the file system.
pub struct File {
	/// Content in bytes.
	pub content: Vec<u8>,
	chunked_content: Option<ChunkedContent>,
	/// Permission level for the file.
	pub permission: Permission
}

impl File {
	fn new(permission: Permission) -> Self {
		Self {
			content: Vec::new(),
			chunked_content: None,
			permission
		}
	}

	fn with_capacity(permission: Permission, capacity: usize) -> Self {
		Self {
			content: Vec::with_capacity(capacity),
			chunked_content: None,
			permission
		}
	}

	fn chunked(permission: Permission) -> Self {
		Self {
			content: Vec::new(),
			chunked_content: Some(ChunkedContent::new()),
			permission
		}
	}

	/// Length of the file in bytes.
	pub fn len(&self) -> usize {
		match &self.chunked_content {
			Some(content) => content.len,
			None => self.content.len()
		}
	}

	/// Returns true when the file has no content.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Copies bytes from `offset` into `out`.
	pub fn read_at(&self, offset: usize, out: &mut [u8]) -> usize {
		match &self.chunked_content {
			Some(content) => content.read_at(offset, out),
			None => {
				let available = self.content.len().saturating_sub(offset);
				let bytes_to_read = core::cmp::min(out.len(), available);
				if bytes_to_read > 0 {
					out[..bytes_to_read]
						.copy_from_slice(&self.content[offset..offset + bytes_to_read]);
				}
				bytes_to_read
			}
		}
	}
}

const FILE_CHUNK_SIZE: usize = 4096;

#[derive(Debug)]
struct ChunkedContent {
	chunks: Vec<Vec<u8>>,
	len: usize
}

impl ChunkedContent {
	fn new() -> Self {
		Self {
			chunks: Vec::new(),
			len: 0
		}
	}

	fn append(&mut self, mut data: &[u8]) {
		while !data.is_empty() {
			let take = core::cmp::min(FILE_CHUNK_SIZE, data.len());
			self.chunks.push(data[..take].to_vec());
			self.len += take;
			data = &data[take..];
		}
	}

	fn read_at(&self, offset: usize, out: &mut [u8]) -> usize {
		if offset >= self.len || out.is_empty() {
			return 0;
		}

		let mut remaining = core::cmp::min(out.len(), self.len - offset);
		let mut file_offset = 0usize;
		let mut out_offset = 0usize;

		for chunk in &self.chunks {
			let chunk_end = file_offset + chunk.len();
			if offset < chunk_end {
				let start = offset.saturating_sub(file_offset);
				let take = core::cmp::min(remaining, chunk.len() - start);
				out[out_offset..out_offset + take].copy_from_slice(&chunk[start..start + take]);
				out_offset += take;
				remaining -= take;
				if remaining == 0 {
					break;
				}
			}
			file_offset = chunk_end;
		}

		out_offset
	}
}

#[derive(Debug)]
/// Structure representing a directory (multiple files + directories)
pub struct Directory {
	pub entries: HashMap<String, Entry>,
	/// Directory permissions
	pub permission: Permission
}

impl Directory {
	fn new(permission: Permission) -> Self {
		Self {
			entries: HashMap::new(),
			permission
		}
	}
}

#[derive(Debug)]
pub enum Entry {
	File(File),
	Directory(Box<Directory>)
}

#[derive(Debug)]
/// Enum for all filesystem errors.
pub enum FsError {
	/// A generic error for unsupported types.
	Generic,

	/// Entry not found
	EntryNotFound,
	/// Target is not a directory.
	NotADirectory,
	/// Target is not a file.
	NotAFile,
	/// Invalid permissions for access.
	PermissionDenied,
	/// File already exists
	AlreadyExists,
	/// Path is invalid.
	InvalidPath,
	/// The directory is currently not empty.
	DirectoryNotEmpty
}

impl fmt::Display for FsError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Generic => write!(f, "An error occurred"),
			Self::EntryNotFound => write!(f, "Entry not found"),
			Self::NotADirectory => write!(f, "Not a directory"),
			Self::NotAFile => write!(f, "Not a file"),
			Self::PermissionDenied => write!(f, "Permission denied"),
			Self::AlreadyExists => write!(f, "Entry already exists"),
			Self::InvalidPath => write!(f, "Invalid path"),
			Self::DirectoryNotEmpty => write!(f, "Directory not empty")
		}
	}
}

impl From<SCFsErrorCode> for FsError {
	fn from(value: SCFsErrorCode) -> Self {
		match value {
			SCFsErrorCode::EntryNotFound => FsError::EntryNotFound,
			SCFsErrorCode::NotADirectory => FsError::NotADirectory,
			SCFsErrorCode::NotAFile => FsError::NotAFile,
			SCFsErrorCode::PermissionDenied => FsError::PermissionDenied,
			SCFsErrorCode::AlreadyExists => FsError::AlreadyExists,
			SCFsErrorCode::DirectoryNotEmpty => FsError::DirectoryNotEmpty,
			_ => FsError::Generic
		}
	}
}

// TODO: put this as a trait.
/// Structure representing a FileSystem.
pub struct FileSystem {
	root: Directory,
	current_path: Vec<String>
}

impl FileSystem {
	/// Create a new `FileSystem`
	pub fn new() -> FileSystem {
		Self {
			root: Directory::new(Permission::all()),
			current_path: Vec::new()
		}
	}

	/// Creates a new file in the current `FileSystem`, unless one is already
	/// created.
	pub fn create_file(&mut self, path: &str, perm: Permission) -> Result<(), FsError> {
		let (dir_components, file_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components.as_slice())?;

		if dir.entries.contains_key(&file_name) {
			return Err(FsError::AlreadyExists);
		}

		dir.entries.insert(file_name, Entry::File(File::new(perm)));
		Ok(())
	}

	/// Creates a new file with reserved content capacity.
	pub fn create_file_with_capacity(
		&mut self,
		path: &str,
		perm: Permission,
		capacity: usize
	) -> Result<(), FsError> {
		let (dir_components, file_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components.as_slice())?;

		if dir.entries.contains_key(&file_name) {
			return Err(FsError::AlreadyExists);
		}

		dir.entries
			.insert(file_name, Entry::File(File::with_capacity(perm, capacity)));
		Ok(())
	}

	/// Creates a file backed by fixed-size chunks.
	pub fn create_chunked_file(&mut self, path: &str, perm: Permission) -> Result<(), FsError> {
		let (dir_components, file_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components.as_slice())?;

		if dir.entries.contains_key(&file_name) {
			return Err(FsError::AlreadyExists);
		}

		dir.entries
			.insert(file_name, Entry::File(File::chunked(perm)));
		Ok(())
	}

	/// Creates a new directory in the current `FileSystem`, unless one is
	/// already created.
	pub fn create_dir(&mut self, path: &str, perm: Permission) -> Result<(), FsError> {
		let (dir_components, dir_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components.as_slice())?;

		if dir.entries.contains_key(&dir_name) {
			return Err(FsError::AlreadyExists);
		}

		dir.entries
			.insert(dir_name, Entry::Directory(Box::new(Directory::new(perm))));
		Ok(())
	}

	/// Writes to a file that already exists
	pub fn write_file(
		&mut self,
		path: &str,
		content: &[u8],
		overwrite: bool
	) -> Result<(), FsError> {
		let file = self.get_file_mut(path)?;
		// check if the file has write permission before appending
		if !file.permission.write {
			return Err(FsError::PermissionDenied);
		}

		if let Some(chunked_content) = file.chunked_content.as_mut() {
			if overwrite {
				chunked_content.chunks.clear();
				chunked_content.len = 0;
			}
			chunked_content.append(content);
			return Ok(());
		}

		// append the new content instead of overwriting
		if overwrite {
			file.content = content.to_vec();
		} else {
			file.content.extend_from_slice(content);
		}
		Ok(())
	}

	pub fn write_file_at(&mut self, path: &str, content: &[u8], offset: usize) -> Result<usize, FsError> {
		let file = self.get_file_mut(path)?;
		if !file.permission.write {
			return Err(FsError::PermissionDenied);
		}
		if file.chunked_content.is_some() {
			let mut full_content = Vec::new();
			let mut off = 0;
			let mut buf = [0u8; FILE_CHUNK_SIZE];
			loop {
				let n = file.read_at(off, &mut buf);
				if n == 0 { break; }
				full_content.extend_from_slice(&buf[..n]);
				off += n;
			}
			file.content = full_content;
			file.chunked_content = None;
		}

		let required_len = offset + content.len();
		if file.content.len() < required_len {
			file.content.resize(required_len, 0);
		}
		file.content[offset..offset + content.len()].copy_from_slice(content);
		
		Ok(content.len())
	}

	/// Appends to a chunked file, creating small fixed-size allocations only.
	pub fn write_file_chunked(&mut self, path: &str, content: &[u8]) -> Result<(), FsError> {
		let file = self.get_file_mut(path)?;
		if !file.permission.write {
			return Err(FsError::PermissionDenied);
		}

		let chunked_content = file.chunked_content.get_or_insert_with(ChunkedContent::new);
		chunked_content.append(content);
		Ok(())
	}

	/// Read the current file.
	// todo: add read permission checks, forgot to add this before.
	pub fn read_file(&self, path: &str) -> Result<&[u8], FsError> {
		let file = self.get_file(path)?;
		if file.chunked_content.is_some() {
			return Err(FsError::NotAFile);
		}
		Ok(&file.content.as_slice())
	}

	/// Returns the file length without requiring contiguous storage.
	pub fn file_len(&self, path: &str) -> Result<usize, FsError> {
		Ok(self.get_file(path)?.len())
	}

	/// Reads bytes from a file into `out` without requiring contiguous storage.
	pub fn read_file_at(
		&self,
		path: &str,
		offset: usize,
		out: &mut [u8]
	) -> Result<usize, FsError> {
		Ok(self.get_file(path)?.read_at(offset, out))
	}

	/// Copies a file into a vector. Prefer `read_file_at` for large files.
	pub fn read_file_to_vec(&self, path: &str) -> Result<Vec<u8>, FsError> {
		let file = self.get_file(path)?;
		match &file.chunked_content {
			Some(_) => {
				let mut out = Vec::new();
				let mut offset = 0usize;
				let mut buf = [0u8; FILE_CHUNK_SIZE];
				loop {
					let n = file.read_at(offset, &mut buf);
					if n == 0 {
						break;
					}
					out.extend_from_slice(&buf[..n]);
					offset += n;
				}
				Ok(out)
			}
			None => Ok(file.content.clone())
		}
	}

	// ----- HELPER FUNCTIONS ----- //

	fn path_components(path: &str) -> Result<Vec<String>, FsError> {
		let mut components = Vec::new();
		for component in path.split('/').filter(|s| !s.is_empty()) {
			if component == "." {
				continue;
			} else if component == ".." {
				if components.pop().is_none() {
					return Err(FsError::InvalidPath);
				}
			} else {
				components.push(component.to_string());
			}
		}
		Ok(components)
	}

	fn split_path(path: &str) -> Result<(Vec<String>, String), FsError> {
		let components = Self::path_components(path)?;
		if components.is_empty() {
			return Err(FsError::InvalidPath);
		}
		let (dir_path, name) = components.split_at(components.len() - 1);
		Ok((dir_path.to_vec(), name[0].clone()))
	}

	fn resolve_path(&self, path: &str) -> Result<Vec<String>, FsError> {
		let base = if path.starts_with('/') {
			Vec::new()
		} else {
			self.current_path.clone()
		};

		let mut components = base;
		components.extend(Self::path_components(path)?);
		Ok(components)
	}

	pub fn get_dir(&self, path: &str) -> Result<&Directory, FsError> {
		let components = self.resolve_path(path)?;
		self.get_dir_from_components(&components.as_slice())
	}

	fn get_dir_from_components(&self, components: &[String]) -> Result<&Directory, FsError> {
		let mut current = &self.root;
		for component in components {
			current = match current.entries.get(component) {
				Some(Entry::Directory(dir)) => &**dir,
				Some(_) => return Err(FsError::NotADirectory),
				None => return Err(FsError::EntryNotFound)
			}
		}
		Ok(current)
	}

	fn get_dir_mut_from_components(
		&mut self,
		components: &[String]
	) -> Result<&mut Directory, FsError> {
		let mut current = &mut self.root;
		for component in components {
			current = match current.entries.get_mut(component) {
				Some(Entry::Directory(dir)) => &mut **dir,
				Some(_) => return Err(FsError::NotADirectory),
				None => return Err(FsError::EntryNotFound)
			}
		}
		Ok(current)
	}

	/// Get a specific file from a file path.
	pub fn get_file(&self, path: &str) -> Result<&File, FsError> {
		let (dir_components, file_name) = Self::split_path(path)?;
		let dir = self.get_dir_from_components(&dir_components.as_slice())?;

		match dir.entries.get(&file_name) {
			Some(Entry::File(file)) => Ok(&file),
			Some(_) => Err(FsError::NotAFile),
			None => Err(FsError::EntryNotFound)
		}
	}

	fn get_file_mut(&mut self, path: &str) -> Result<&mut File, FsError> {
		let (dir_components, file_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components.as_slice())?;

		match dir.entries.get_mut(&file_name) {
			Some(Entry::File(file)) => Ok(&mut *file),
			Some(_) => Err(FsError::NotAFile),
			None => Err(FsError::EntryNotFound)
		}
	}

	/// List all contents of a specified path.
	pub fn list_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
		let dir = self.get_dir(path)?;
		Ok(dir.entries.keys().cloned().collect())
	}

	/// List all contents of a specified path and their type.
	pub fn list_dir_entry_types(&self, path: &str) -> Result<Vec<String>, FsError> {
		let dir = self.get_dir(path)?;
		Ok(dir
			.entries
			.values()
			.map(|entry| match entry {
				Entry::File(_) => "File".to_string(),
				Entry::Directory(_) => "Directory".to_string()
			})
			.collect())
	}

	/// If a path is a directory.
	pub fn is_dir(&self, path: &str) -> bool {
		let components = match self.resolve_path(path) {
			Ok(c) => c,
			Err(_) => return false
		};

		// special case for root directory
		if components.is_empty() {
			return true;
		}

		match self.get_dir_from_components(&components[..components.len() - 1]) {
			Ok(parent_dir) => {
				if let Some(entry) = parent_dir.entries.get(&components[components.len() - 1]) {
					matches!(entry, Entry::Directory(_))
				} else {
					false
				}
			}
			Err(_) => false
		}
	}

	/// Remove the item at the specified path.
	pub fn remove(&mut self, path: &str, del_dir: bool, recursive: bool) -> Result<(), FsError> {
		// split the path into parent components and the name of the entry.
		let (parent_components, name) = Self::split_path(path)?;
		let parent_dir = self.get_dir_mut_from_components(&parent_components.as_slice())?;
		// remove entry from parent's entries to gain ownership.
		let entry = parent_dir
			.entries
			.remove(&name)
			.ok_or(FsError::EntryNotFound)?;

		match entry {
			Entry::Directory(mut dir_box) => {
				if !del_dir {
					// caller did not intend to delete a directory.
					parent_dir.entries.insert(name, Entry::Directory(dir_box));
					return Err(FsError::NotADirectory);
				}

				if !recursive && !dir_box.entries.is_empty() {
					// recursive deletion not enabled and directory is not empty.
					parent_dir.entries.insert(name, Entry::Directory(dir_box));
					return Err(FsError::DirectoryNotEmpty);
				}

				if recursive {
					Self::recursive_remove(&mut dir_box);
				}
				// with recursive deletion (or if empty), dropping dir_box completes removal.
				Ok(())
			}
			Entry::File(_) => Ok(())
		}
	}

	fn recursive_remove(dir: &mut Directory) {
		// recursively remove all entries inside the directory.
		// first collect keys to avoid mutable borrow issues.
		let keys: Vec<String> = dir.entries.keys().cloned().collect();
		for key in keys {
			if let Some(entry) = dir.entries.get_mut(&key)
				&& let Entry::Directory(ref mut subdir) = *entry
			{
				Self::recursive_remove(subdir);
			}
		}
		// clear all entries from the directory.
		dir.entries.clear();
	}

	/// If the specified path exists.
	pub fn exists(&self, path: &str) -> bool {
		let components = match self.resolve_path(path) {
			Ok(c) => c,
			Err(_) => return false
		};

		if components.is_empty() {
			return true;
		}

		if let Ok(parent_dir) = self.get_dir_from_components(&components[..components.len() - 1]) {
			parent_dir
				.entries
				.contains_key(&components[components.len() - 1])
		} else {
			false
		}
	}
}

impl Default for FileSystem {
	fn default() -> Self {
		Self::new()
	}
}

/// Setup system files.
// this will be replaced with a one time call if the system files are not there
// on ATA disk.
pub fn setup_system_files(mut fs: FileSystem) {
	fs.create_dir("/logs", Permission::all()).unwrap();
	fs.create_dir("/proc", Permission::read()).unwrap();
	fs.create_dir("/apps", Permission::all()).unwrap();
	fs.create_dir("/init", Permission::read()).unwrap();
	fs.create_dir("/var", Permission::readwrite()).unwrap();
	fs.create_dir("/var/lib", Permission::readwrite()).unwrap();

	for elf in USERSPACE_ELFS {
		let path = if elf.name == "nush" || elf.name == "ktest" {
			"/init/".to_string() + elf.name + ".elf"
		} else {
			"/apps/".to_string() + elf.name + ".elf"
		};

		if !fs.exists(&path) {
			fs.create_file(&path, Permission::all()).unwrap();
		}

		fs.write_file(&path, elf.bytes, true).unwrap();
	}

	fs.create_file("/init/startup.ini", Permission::all());
	fs.write_file(
		"/init/startup.ini",
		b"
[Startup]
init_path=\"/init/nush.elf\"
args=\"\"	
",
		false
	);

	init_fs(fs);
}

#[cfg(feature = "test")]
pub mod tests {
	use crate::{
		alloc::string::ToString,
		fs::{
			SCFsErrorCode,
			SCPermission,
			ramfs::{
				ChunkedContent,
				Directory,
				FILE_CHUNK_SIZE,
				File,
				FileSystem,
				FsError,
				Permission
			}
		},
		tassert,
		tassert_eq,
		testing::ktest::TestError
	};

	pub fn test_permission_all() -> Result<(), TestError> {
		let p = Permission::all();
		tassert!(p.read && p.write && p.execute);
		Ok(())
	}
	crate::create_test!(test_permission_all);

	pub fn test_permission_read() -> Result<(), TestError> {
		let p = Permission::read();
		tassert!(p.read && !p.write && !p.execute);
		Ok(())
	}
	crate::create_test!(test_permission_read);

	pub fn test_permission_write() -> Result<(), TestError> {
		let p = Permission::write();
		tassert!(!p.read && p.write && !p.execute);
		Ok(())
	}
	crate::create_test!(test_permission_write);

	pub fn test_permission_none() -> Result<(), TestError> {
		let p = Permission::none();
		tassert!(!p.read && !p.write && !p.execute);
		Ok(())
	}
	crate::create_test!(test_permission_none);

	pub fn test_permission_execute() -> Result<(), TestError> {
		let p = Permission::execute();
		tassert!(!p.read && !p.write && p.execute);
		Ok(())
	}
	crate::create_test!(test_permission_execute);

	pub fn test_permission_readwrite() -> Result<(), TestError> {
		let p = Permission::readwrite();
		tassert!(p.read && p.write && !p.execute);
		Ok(())
	}
	crate::create_test!(test_permission_readwrite);

	pub fn test_permission_from_scpermission() -> Result<(), TestError> {
		let scp = SCPermission {
			read: 1,
			write: 0,
			execute: 1
		};
		let p = Permission::from(scp);
		tassert!(p.read && !p.write && p.execute);
		Ok(())
	}
	crate::create_test!(test_permission_from_scpermission);

	pub fn test_file_new_and_is_empty() -> Result<(), TestError> {
		let f = File::new(Permission::all());
		tassert!(f.is_empty());
		tassert_eq!(f.len(), 0);
		Ok(())
	}
	crate::create_test!(test_file_new_and_is_empty);

	pub fn test_file_with_capacity() -> Result<(), TestError> {
		let f = File::with_capacity(Permission::all(), 1024);
		tassert!(f.is_empty());
		tassert!(f.content.capacity() >= 1024);
		Ok(())
	}
	crate::create_test!(test_file_with_capacity);

	pub fn test_file_chunked() -> Result<(), TestError> {
		let f = File::chunked(Permission::all());
		tassert!(f.chunked_content.is_some());
		tassert!(f.is_empty());
		Ok(())
	}
	crate::create_test!(test_file_chunked);

	pub fn test_file_read_at() -> Result<(), TestError> {
		let mut f = File::new(Permission::all());
		f.content.extend_from_slice(b"hello world");
		let mut buf = [0u8; 5];
		let read = f.read_at(6, &mut buf);
		tassert_eq!(read, 5);
		tassert_eq!(&buf, b"world");
		Ok(())
	}
	crate::create_test!(test_file_read_at);

	pub fn test_file_read_at_out_of_bounds() -> Result<(), TestError> {
		let mut f = File::new(Permission::all());
		f.content.extend_from_slice(b"hello");
		let mut buf = [0u8; 10];
		let read = f.read_at(10, &mut buf);
		tassert_eq!(read, 0);
		let read2 = f.read_at(3, &mut buf);
		tassert_eq!(read2, 2);
		tassert_eq!(&buf[..2], b"lo");
		Ok(())
	}
	crate::create_test!(test_file_read_at_out_of_bounds);

	pub fn test_chunked_content_append_and_read() -> Result<(), TestError> {
		let mut cc = ChunkedContent::new();
		cc.append(b"hello world");
		tassert_eq!(cc.len, 11);
		let mut buf = [0u8; 5];
		let read = cc.read_at(6, &mut buf);
		tassert_eq!(read, 5);
		tassert_eq!(&buf, b"world");
		Ok(())
	}
	crate::create_test!(test_chunked_content_append_and_read);

	pub fn test_chunked_content_append_multiple_chunks() -> Result<(), TestError> {
		let mut cc = ChunkedContent::new();
		let data = vec![1u8; 5000];
		cc.append(&data);
		tassert_eq!(cc.len, 5000);
		tassert_eq!(cc.chunks.len(), 2);
		tassert_eq!(cc.chunks[0].len(), FILE_CHUNK_SIZE);
		tassert_eq!(cc.chunks[1].len(), 5000 - FILE_CHUNK_SIZE);
		Ok(())
	}
	crate::create_test!(test_chunked_content_append_multiple_chunks);

	pub fn test_chunked_content_read_at_spanning_chunks() -> Result<(), TestError> {
		let mut cc = ChunkedContent::new();
		let data1 = vec![1u8; FILE_CHUNK_SIZE];
		let data2 = vec![2u8; FILE_CHUNK_SIZE];
		cc.append(&data1);
		cc.append(&data2);

		let mut buf = [0u8; 10];
		let offset = FILE_CHUNK_SIZE - 5;
		let read = cc.read_at(offset, &mut buf);
		tassert_eq!(read, 10);
		tassert_eq!(&buf[..5], &[1u8; 5]);
		tassert_eq!(&buf[5..], &[2u8; 5]);
		Ok(())
	}
	crate::create_test!(test_chunked_content_read_at_spanning_chunks);

	pub fn test_directory_new() -> Result<(), TestError> {
		let d = Directory::new(Permission::all());
		tassert!(d.entries.is_empty());
		Ok(())
	}
	crate::create_test!(test_directory_new);

	pub fn test_fs_error_display() -> Result<(), TestError> {
		tassert_eq!(format!("{}", FsError::Generic), "An error occurred");
		tassert_eq!(format!("{}", FsError::EntryNotFound), "Entry not found");
		tassert_eq!(format!("{}", FsError::NotADirectory), "Not a directory");
		tassert_eq!(format!("{}", FsError::NotAFile), "Not a file");
		tassert_eq!(
			format!("{}", FsError::PermissionDenied),
			"Permission denied"
		);
		tassert_eq!(
			format!("{}", FsError::AlreadyExists),
			"Entry already exists"
		);
		tassert_eq!(format!("{}", FsError::InvalidPath), "Invalid path");
		tassert_eq!(
			format!("{}", FsError::DirectoryNotEmpty),
			"Directory not empty"
		);
		Ok(())
	}
	crate::create_test!(test_fs_error_display);

	pub fn test_fs_error_from_sc_error_code() -> Result<(), TestError> {
		tassert!(matches!(
			FsError::from(SCFsErrorCode::EntryNotFound),
			FsError::EntryNotFound
		));
		tassert!(matches!(
			FsError::from(SCFsErrorCode::NotADirectory),
			FsError::NotADirectory
		));
		tassert!(matches!(
			FsError::from(SCFsErrorCode::NotAFile),
			FsError::NotAFile
		));
		tassert!(matches!(
			FsError::from(SCFsErrorCode::PermissionDenied),
			FsError::PermissionDenied
		));
		tassert!(matches!(
			FsError::from(SCFsErrorCode::AlreadyExists),
			FsError::AlreadyExists
		));
		tassert!(matches!(
			FsError::from(SCFsErrorCode::DirectoryNotEmpty),
			FsError::DirectoryNotEmpty
		));
		Ok(())
	}
	crate::create_test!(test_fs_error_from_sc_error_code);

	pub fn test_filesystem_new() -> Result<(), TestError> {
		let fs = FileSystem::new();
		tassert!(fs.root.entries.is_empty());
		tassert!(fs.current_path.is_empty());
		Ok(())
	}
	crate::create_test!(test_filesystem_new);

	pub fn test_path_components() -> Result<(), TestError> {
		let comps = FileSystem::path_components("a/b/c").unwrap();
		tassert_eq!(comps, vec!["a", "b", "c"]);
		Ok(())
	}
	crate::create_test!(test_path_components);

	pub fn test_path_components_with_dots() -> Result<(), TestError> {
		let comps = FileSystem::path_components("a/./b/../c").unwrap();
		tassert_eq!(comps, vec!["a", "c"]);
		Ok(())
	}
	crate::create_test!(test_path_components_with_dots);

	pub fn test_path_components_invalid_parent() -> Result<(), TestError> {
		let res = FileSystem::path_components("a/../../b");
		tassert!(matches!(res, Err(FsError::InvalidPath)));
		Ok(())
	}
	crate::create_test!(test_path_components_invalid_parent);

	pub fn test_split_path() -> Result<(), TestError> {
		let (dir, name) = FileSystem::split_path("a/b/c.txt").unwrap();
		tassert_eq!(dir, vec!["a", "b"]);
		tassert_eq!(name, "c.txt");
		Ok(())
	}
	crate::create_test!(test_split_path);

	pub fn test_split_path_invalid() -> Result<(), TestError> {
		let res = FileSystem::split_path("");
		tassert!(matches!(res, Err(FsError::InvalidPath)));
		Ok(())
	}
	crate::create_test!(test_split_path_invalid);

	pub fn test_resolve_path_absolute_and_relative() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.current_path = alloc::vec!["a".to_string(), "b".to_string()];

		let abs = fs.resolve_path("/c/d").unwrap();
		tassert_eq!(abs, vec!["c", "d"]);

		let rel = fs.resolve_path("c/d").unwrap();
		tassert_eq!(rel, vec!["a", "b", "c", "d"]);
		Ok(())
	}
	crate::create_test!(test_resolve_path_absolute_and_relative);

	pub fn test_create_file_and_exists() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file("test.txt", Permission::all()).unwrap();
		tassert!(fs.exists("test.txt"));
		Ok(())
	}
	crate::create_test!(test_create_file_and_exists);

	pub fn test_create_file_already_exists() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file("test.txt", Permission::all()).unwrap();
		let res = fs.create_file("test.txt", Permission::all());
		tassert!(matches!(res, Err(FsError::AlreadyExists)));
		Ok(())
	}
	crate::create_test!(test_create_file_already_exists);

	pub fn test_create_file_with_capacity() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file_with_capacity("test.txt", Permission::all(), 1024)
			.unwrap();
		let file = fs.get_file("test.txt").unwrap();
		tassert!(file.content.capacity() >= 1024);
		Ok(())
	}
	crate::create_test!(test_create_file_with_capacity);

	pub fn test_create_chunked_file() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_chunked_file("test.txt", Permission::all())
			.unwrap();
		let file = fs.get_file("test.txt").unwrap();
		tassert!(file.chunked_content.is_some());
		Ok(())
	}
	crate::create_test!(test_create_chunked_file);

	pub fn test_create_dir() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_dir("dir", Permission::all()).unwrap();
		tassert!(fs.is_dir("dir"));
		Ok(())
	}
	crate::create_test!(test_create_dir);

	pub fn test_create_dir_already_exists() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_dir("dir", Permission::all()).unwrap();
		let res = fs.create_dir("dir", Permission::all());
		tassert!(matches!(res, Err(FsError::AlreadyExists)));
		Ok(())
	}
	crate::create_test!(test_create_dir_already_exists);

	pub fn test_write_file_and_read_file() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file("test.txt", Permission::all()).unwrap();
		fs.write_file("test.txt", b"hello", true).unwrap();
		let content = fs.read_file("test.txt").unwrap();
		tassert_eq!(content, b"hello");
		Ok(())
	}
	crate::create_test!(test_write_file_and_read_file);

	pub fn test_write_file_append() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file("test.txt", Permission::all()).unwrap();
		fs.write_file("test.txt", b"hello", true).unwrap();
		fs.write_file("test.txt", b" world", false).unwrap();
		let content = fs.read_file("test.txt").unwrap();
		tassert_eq!(content, b"hello world");
		Ok(())
	}
	crate::create_test!(test_write_file_append);

	pub fn test_write_file_overwrite() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file("test.txt", Permission::all()).unwrap();
		fs.write_file("test.txt", b"hello", true).unwrap();
		fs.write_file("test.txt", b"world", true).unwrap();
		let content = fs.read_file("test.txt").unwrap();
		tassert_eq!(content, b"world");
		Ok(())
	}
	crate::create_test!(test_write_file_overwrite);

	pub fn test_write_file_permission_denied() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file("test.txt", Permission::read()).unwrap();
		let res = fs.write_file("test.txt", b"data", true);
		tassert!(matches!(res, Err(FsError::PermissionDenied)));
		Ok(())
	}
	crate::create_test!(test_write_file_permission_denied);

	pub fn test_write_file_chunked() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_chunked_file("test.txt", Permission::all())
			.unwrap();
		fs.write_file_chunked("test.txt", b"hello").unwrap();
		let len = fs.file_len("test.txt").unwrap();
		tassert_eq!(len, 5);
		Ok(())
	}
	crate::create_test!(test_write_file_chunked);

	pub fn test_read_file_chunked_returns_error() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_chunked_file("test.txt", Permission::all())
			.unwrap();
		fs.write_file_chunked("test.txt", b"data").unwrap();
		let res = fs.read_file("test.txt");
		tassert!(matches!(res, Err(FsError::NotAFile)));
		Ok(())
	}
	crate::create_test!(test_read_file_chunked_returns_error);

	pub fn test_file_len() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file("test.txt", Permission::all()).unwrap();
		fs.write_file("test.txt", b"hello", true).unwrap();
		let len = fs.file_len("test.txt").unwrap();
		tassert_eq!(len, 5);
		Ok(())
	}
	crate::create_test!(test_file_len);

	pub fn test_read_file_at() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file("test.txt", Permission::all()).unwrap();
		fs.write_file("test.txt", b"hello world", true).unwrap();
		let mut buf = [0u8; 5];
		let read = fs.read_file_at("test.txt", 6, &mut buf).unwrap();
		tassert_eq!(read, 5);
		tassert_eq!(&buf, b"world");
		Ok(())
	}
	crate::create_test!(test_read_file_at);

	pub fn test_read_file_to_vec() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file("test.txt", Permission::all()).unwrap();
		fs.write_file("test.txt", b"hello", true).unwrap();
		let vec = fs.read_file_to_vec("test.txt").unwrap();
		tassert_eq!(vec, b"hello");
		Ok(())
	}
	crate::create_test!(test_read_file_to_vec);

	pub fn test_read_file_to_vec_chunked() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_chunked_file("test.txt", Permission::all())
			.unwrap();
		fs.write_file_chunked("test.txt", b"hello").unwrap();
		let vec = fs.read_file_to_vec("test.txt").unwrap();
		tassert_eq!(vec, b"hello");
		Ok(())
	}
	crate::create_test!(test_read_file_to_vec_chunked);

	pub fn test_get_dir_and_list_dir() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_dir("dir", Permission::all()).unwrap();
		fs.create_file("dir/a.txt", Permission::all()).unwrap();
		fs.create_file("dir/b.txt", Permission::all()).unwrap();
		let mut list = fs.list_dir("dir").unwrap();
		list.sort();
		tassert_eq!(list, vec!["a.txt", "b.txt"]);
		Ok(())
	}
	crate::create_test!(test_get_dir_and_list_dir);

	pub fn test_list_dir_entry_types() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_dir("dir", Permission::all()).unwrap();
		fs.create_file("dir/a.txt", Permission::all()).unwrap();
		fs.create_dir("dir/sub", Permission::all()).unwrap();
		let mut list = fs.list_dir_entry_types("dir").unwrap();
		list.sort();
		tassert_eq!(list, vec!["Directory", "File"]);
		Ok(())
	}
	crate::create_test!(test_list_dir_entry_types);

	pub fn test_is_dir() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_dir("dir", Permission::all()).unwrap();
		fs.create_file("file.txt", Permission::all()).unwrap();
		tassert!(fs.is_dir("dir"));
		tassert!(!fs.is_dir("file.txt"));
		tassert!(fs.is_dir("/"));
		Ok(())
	}
	crate::create_test!(test_is_dir);

	pub fn test_remove_file() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_file("test.txt", Permission::all()).unwrap();
		fs.remove("test.txt", false, false).unwrap();
		tassert!(!fs.exists("test.txt"));
		Ok(())
	}
	crate::create_test!(test_remove_file);

	pub fn test_remove_empty_dir() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_dir("dir", Permission::all()).unwrap();
		fs.remove("dir", true, false).unwrap();
		tassert!(!fs.exists("dir"));
		Ok(())
	}
	crate::create_test!(test_remove_empty_dir);

	pub fn test_remove_non_empty_dir_without_recursive() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_dir("dir", Permission::all()).unwrap();
		fs.create_file("dir/file.txt", Permission::all()).unwrap();
		let res = fs.remove("dir", true, false);
		tassert!(matches!(res, Err(FsError::DirectoryNotEmpty)));
		tassert!(fs.exists("dir"));
		Ok(())
	}
	crate::create_test!(test_remove_non_empty_dir_without_recursive);

	pub fn test_remove_non_empty_dir_with_recursive() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_dir("dir", Permission::all()).unwrap();
		fs.create_dir("dir/sub", Permission::all()).unwrap();
		fs.create_file("dir/sub/file.txt", Permission::all())
			.unwrap();
		fs.remove("dir", true, true).unwrap();
		tassert!(!fs.exists("dir"));
		Ok(())
	}
	crate::create_test!(test_remove_non_empty_dir_with_recursive);

	pub fn test_remove_dir_as_file_fails() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		fs.create_dir("dir", Permission::all()).unwrap();
		let res = fs.remove("dir", false, false);
		tassert!(matches!(res, Err(FsError::NotADirectory)));
		tassert!(fs.exists("dir"));
		Ok(())
	}
	crate::create_test!(test_remove_dir_as_file_fails);

	pub fn test_remove_non_existent() -> Result<(), TestError> {
		let mut fs = FileSystem::new();
		let res = fs.remove("nonexistent", false, false);
		tassert!(matches!(res, Err(FsError::EntryNotFound)));
		Ok(())
	}
	crate::create_test!(test_remove_non_existent);
}
