/*
RamFS implementation for the kernel.
*/

use alloc::{
	boxed::Box,
	string::{String, ToString},
	vec::Vec
};
use core::str;

use hashbrown::HashMap;

use crate::errors::KernelError;
// Import KernelError and error codes
use crate::errors::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Permission {
	pub read: bool,
	pub write: bool,
	pub execute: bool
}

impl Permission {
	pub fn all() -> Self {
		Self {
			read: true,
			write: true,
			execute: true
		}
	}
}

#[derive(Debug, PartialEq, Clone)]
pub struct File {
	pub content: Vec<u8>,
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

#[derive(Debug, PartialEq, Clone)]
pub struct Directory {
	entries: HashMap<String, Entry>,
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

#[derive(Debug, PartialEq, Clone)]
pub enum Entry {
	File(File),
	Directory(Box<Directory>)
}

pub struct FileSystem {
	root: Directory,
	current_path: Vec<String>
}

impl FileSystem {
	pub fn new() -> Self {
		Self {
			root: Directory::new(Permission::all()),
			current_path: Vec::new()
		}
	}

	/// Creates a new file at the specified path with given permissions.
	pub fn create_file(&mut self, path: &str, perm: Permission) -> Result<(), KernelError> {
		let (dir_components, file_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components)?;

		if dir.entries.contains_key(&file_name) {
			return Err(KernelError::FileSystemError(FS_FILE_EXISTS));
		}

		dir.entries.insert(file_name, Entry::File(File::new(perm)));
		Ok(())
	}

	/// Creates a new directory at the specified path with given permissions.
	pub fn create_dir(&mut self, path: &str, perm: Permission) -> Result<(), KernelError> {
		let (dir_components, dir_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components)?;

		if dir.entries.contains_key(&dir_name) {
			return Err(KernelError::FileSystemError(FS_FILE_EXISTS));
		}

		dir.entries
			.insert(dir_name, Entry::Directory(Box::new(Directory::new(perm))));
		Ok(())
	}

	/// Writes content to the file at the specified path.
	pub fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), KernelError> {
		let file = self.get_file_mut(path)?;
		if !file.permission.write {
			return Err(KernelError::FileSystemError(FS_FILE_INVALID_PERMISSION));
		}
		file.content.extend_from_slice(content);
		Ok(())
	}

	/// Reads the content of the file at the specified path.
	pub fn read_file(&self, path: &str) -> Result<&[u8], KernelError> {
		let file = self.get_file(path)?;
		Ok(&file.content)
	}

	// Helper Functions

	/// Splits a path into components.
	fn path_components(path: &str) -> Result<Vec<String>, KernelError> {
		let mut components = Vec::new();
		for component in path.split('/').filter(|s| !s.is_empty()) {
			if component == "." {
				continue;
			} else if component == ".." {
				if components.pop().is_none() {
					return Err(KernelError::FileSystemError(FS_FILE_INVALID_PATH));
				}
			} else {
				components.push(component.to_string());
			}
		}
		Ok(components)
	}

	/// Splits a path into directory components and the final name.
	fn split_path(path: &str) -> Result<(Vec<String>, String), KernelError> {
		let components = Self::path_components(path)?;
		if components.is_empty() {
			return Err(KernelError::FileSystemError(FS_FILE_INVALID_PATH));
		}
		let (dir_path, name) = components.split_at(components.len() - 1);
		Ok((dir_path.to_vec(), name[0].clone()))
	}

	/// Resolves a path relative to the current directory or as an absolute
	/// path.
	fn resolve_path(&self, path: &str) -> Result<Vec<String>, KernelError> {
		let base = if path.starts_with('/') {
			Vec::new()
		} else {
			self.current_path.clone()
		};

		let mut components = base;
		components.extend(Self::path_components(path)?);
		Ok(components)
	}

	/// Gets a directory reference from a path.
	fn get_dir(&self, path: &str) -> Result<&Directory, KernelError> {
		let components = self.resolve_path(path)?;
		self.get_dir_from_components(&components)
	}

	/// Gets a mutable directory reference from a path.
	pub fn get_dir_mut(&mut self, path: &str) -> Result<&mut Directory, KernelError> {
		let components = self.resolve_path(path)?;
		self.get_dir_mut_from_components(&components)
	}

	/// Gets a directory reference from path components.
	fn get_dir_from_components(&self, components: &[String]) -> Result<&Directory, KernelError> {
		let mut current = &self.root;
		for component in components {
			current = match current.entries.get(component) {
				Some(Entry::Directory(dir)) => &**dir,
				Some(_) => return Err(KernelError::FileSystemError(FS_FILE_INVALID_PATH)),
				None => return Err(KernelError::FileSystemError(FS_FILE_NOT_FOUND))
			};
		}
		Ok(current)
	}

	/// Gets a mutable directory reference from path components.
	fn get_dir_mut_from_components(
		&mut self,
		components: &[String]
	) -> Result<&mut Directory, KernelError> {
		let mut current = &mut self.root;
		for component in components {
			current = match current.entries.get_mut(component) {
				Some(Entry::Directory(dir)) => &mut **dir,
				Some(_) => return Err(KernelError::FileSystemError(FS_FILE_INVALID_PATH)),
				None => return Err(KernelError::FileSystemError(FS_FILE_NOT_FOUND))
			};
		}
		Ok(current)
	}

	/// Gets a file reference from a path.
	pub fn get_file(&self, path: &str) -> Result<&File, KernelError> {
		let (dir_components, file_name) = Self::split_path(path)?;
		let dir = self.get_dir_from_components(&dir_components)?;

		match dir.entries.get(&file_name) {
			Some(Entry::File(file)) => Ok(&file),
			Some(_) => Err(KernelError::FileSystemError(FS_FILE_INVALID_PATH)),
			None => Err(KernelError::FileSystemError(FS_FILE_NOT_FOUND))
		}
	}

	fn get_file_mut(&mut self, path: &str) -> Result<&mut File, KernelError> {
		let (dir_components, file_name) = Self::split_path(path)?;
		let dir = self.get_dir_mut_from_components(&dir_components)?;

		match dir.entries.get_mut(&file_name) {
			Some(Entry::File(file)) => Ok(&mut *file),
			Some(_) => Err(KernelError::FileSystemError(FS_FILE_INVALID_PATH)),
			None => Err(KernelError::FileSystemError(FS_FILE_NOT_FOUND))
		}
	}

	/// Lists the entries in a directory.
	pub fn list_dir(&self, path: &str) -> Result<Vec<String>, KernelError> {
		let dir = self.get_dir(path)?;
		Ok(dir.entries.keys().cloned().collect())
	}

	/// Lists the entry types in a directory.
	pub fn list_dir_entry_types(&self, path: &str) -> Result<Vec<String>, KernelError> {
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

	/// Checks if a path is a directory.
	pub fn is_dir(&self, path: &str) -> bool {
		let components = match self.resolve_path(path) {
			Ok(c) => c,
			Err(_) => return false
		};

		if components.is_empty() {
			return true; // Root is a directory
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

	/// Removes a file or directory at the specified path.
	pub fn remove(
		&mut self,
		path: &str,
		del_dir: bool,
		recursive: bool
	) -> Result<(), KernelError> {
		let (parent_components, name) = Self::split_path(path)?;
		let parent_dir = self.get_dir_mut_from_components(&parent_components)?;
		let entry = parent_dir
			.entries
			.remove(&name)
			.ok_or(KernelError::FileSystemError(FS_FILE_NOT_FOUND))?;

		match entry {
			Entry::Directory(mut dir_box) => {
				if !del_dir {
					parent_dir.entries.insert(name, Entry::Directory(dir_box));
					return Err(KernelError::FileSystemError(FS_DELETE_ERROR));
				}

				if !recursive && !dir_box.entries.is_empty() {
					parent_dir.entries.insert(name, Entry::Directory(dir_box));
					return Err(KernelError::FileSystemError(FS_DELETE_ERROR));
				}

				if recursive {
					Self::recursive_remove(&mut *dir_box);
				}
				Ok(())
			}
			Entry::File(_) => Ok(())
		}
	}

	/// Recursively removes all entries in a directory.
	fn recursive_remove(dir: &mut Directory) {
		let keys: Vec<String> = dir.entries.keys().cloned().collect();
		for key in keys {
			if let Some(entry) = dir.entries.get_mut(&key) {
				if let Entry::Directory(ref mut subdir) = *entry {
					Self::recursive_remove(subdir);
				}
			}
		}
		dir.entries.clear();
	}

	/// Checks if a path exists.
	pub fn exists(&self, path: &str) -> bool {
		let components = match self.resolve_path(path) {
			Ok(c) => c,
			Err(_) => return false
		};

		if components.is_empty() {
			return true; // Root exists
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
