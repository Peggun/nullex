//!
//! ramfs.rs
//!
//! RamFS implementation for the kernel.
//!

use alloc::{
	boxed::Box,
	string::{String, ToString},
	vec::Vec
};
use core::{fmt, str};

use hashbrown::HashMap;

use crate::fs::init_fs;

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
}

#[derive(Debug)]
/// Structure representing a file in the file system.
pub struct File {
	/// Content in bytes.
	pub content: Vec<u8>,
	/// Permission level for the file.
	pub permission: Permission
}

impl File {
	fn new(permission: Permission) -> Self {
		Self {
			content: Vec::new(),
			permission
		}
	}
}

#[derive(Debug)]
/// Structure representing a directory (multiple files + directories)
pub struct Directory {
	entries: HashMap<String, Entry>,
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
enum Entry {
	File(File),
	Directory(Box<Directory>)
}

#[derive(Debug)]
/// Enum for all filesystem errors.
pub enum FsError {
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

	/// Creates a new file in the current `FileSystem`, unless one is already created.
	pub fn create_file(&mut self, path: &str, perm: Permission) -> Result<(), FsError> {
		let (dir_components, file_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components)?;

		if dir.entries.contains_key(&file_name) {
			return Err(FsError::AlreadyExists);
		}

		dir.entries.insert(file_name, Entry::File(File::new(perm)));
		Ok(())
	}

	/// Creates a new directory in the current `FileSystem`, unless one is already created.
	pub fn create_dir(&mut self, path: &str, perm: Permission) -> Result<(), FsError> {
		let (dir_components, dir_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components)?;

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
		// append the new content instead of overwriting
		if overwrite {
			file.content = content.to_vec();
		} else {
			file.content.extend_from_slice(content);
		}
		Ok(())
	}

	/// Read the current file.
	// todo: add read permission checks, forgot to add this before.
	pub fn read_file(&self, path: &str) -> Result<&[u8], FsError> {
		let file = self.get_file(path)?;
		Ok(&file.content)
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

	fn get_dir(&self, path: &str) -> Result<&Directory, FsError> {
		let components = self.resolve_path(path)?;
		self.get_dir_from_components(&components)
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
		let dir = self.get_dir_from_components(&dir_components)?;

		match dir.entries.get(&file_name) {
			Some(Entry::File(file)) => Ok(file),
			Some(_) => Err(FsError::NotAFile),
			None => Err(FsError::EntryNotFound)
		}
	}

	fn get_file_mut(&mut self, path: &str) -> Result<&mut File, FsError> {
		let (dir_components, file_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components)?;

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
		let parent_dir = self.get_dir_mut_from_components(&parent_components)?;
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
pub fn setup_system_files(mut fs: FileSystem) {
	fs.create_dir("/logs", Permission::all()).unwrap();
	fs.create_dir("/proc", Permission::read()).unwrap();

	init_fs(fs);
}