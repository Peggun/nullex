// ramfs.rs
use core::{fmt, str};
use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use hashbrown::HashMap;

use crate::println;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Permission {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl Permission {
    pub fn all() -> Self {
        Self {
            read: true,
            write: true,
            execute: true,
        }
    }
}

#[derive(Debug)]
pub struct File {
    content: Vec<u8>,
    permission: Permission,
}

impl File {
    fn new(permission: Permission) -> Self {
        Self {
            content: Vec::new(),
            permission,
        }
    }
}

#[derive(Debug)]
pub struct Directory {
    entries: HashMap<String, Entry>,
    permission: Permission,
}

impl Directory {
    fn new(permission: Permission) -> Self {
        Self {
            entries: HashMap::new(),
            permission,
        }
    }
}

#[derive(Debug)]
pub enum Entry {
    File(File),
    Directory(Box<Directory>),
}

#[derive(Debug)]
pub enum FsError {
    EntryNotFound,
    NotADirectory,
    NotAFile,
    PermissionDenied,
    AlreadyExists,
    InvalidPath,
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
        }
    }
}

pub struct FileSystem {
    root: Directory,
    current_path: Vec<String>,
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            root: Directory::new(Permission::all()),
            current_path: Vec::new(),
        }
    }

    pub fn create_file(&mut self, path: &str, perm: Permission) -> Result<(), FsError> {
        let (dir_components, file_name) = Self::split_path(path)?;
        let dir = self.get_dir_mut_from_components(&dir_components)?;
        
        if dir.entries.contains_key(&file_name) {
            return Err(FsError::AlreadyExists);
        }
        
        dir.entries.insert(
            file_name,
            Entry::File(File::new(perm))
        );
        Ok(())
    }

    pub fn create_dir(&mut self, path: &str, perm: Permission) -> Result<(), FsError> {
        let (dir_components, dir_name) = Self::split_path(path)?;
        let dir = self.get_dir_mut_from_components(&dir_components)?;
        
        if dir.entries.contains_key(&dir_name) {
            return Err(FsError::AlreadyExists);
        }
        
        dir.entries.insert(
            dir_name,
            Entry::Directory(Box::new(Directory::new(perm)))
        );
        Ok(())
    }

    pub fn write_file(&mut self, path: &str, content: &[u8]) -> Result<(), FsError> {
        let file = self.get_file_mut(path)?;
        file.content = content.to_vec();
        Ok(())
    }

    pub fn read_file(&self, path: &str) -> Result<&[u8], FsError> {
        let file = self.get_file(path)?;
        Ok(&file.content)
    }

    // Command implementations
    pub fn ls(&self, path: &str) -> Result<(), FsError> {
        let dir = self.get_dir(path)?;
        for (name, entry) in &dir.entries {
            let typ = match entry {
                Entry::File(_) => "File",
                Entry::Directory(_) => "Dir",
            };
            println!("{} [{}]", name, typ);
        }
        Ok(())
    }

    pub fn cd(&mut self, path: &str) -> Result<(), FsError> {
        let new_path = self.resolve_path(path)?;
        self.current_path = new_path;
        Ok(())
    }

    pub fn cat(&self, path: &str) -> Result<(), FsError> {
        let content = self.read_file(path)?;
        if let Ok(text) = str::from_utf8(content) {
            println!("{}", text);
        } else {
            println!("<binary data: {} bytes>", content.len());
        }
        Ok(())
    }

    // Helper functions
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

    fn get_dir_mut(&mut self, path: &str) -> Result<&mut Directory, FsError> {
        let components = self.resolve_path(path)?;
        self.get_dir_mut_from_components(&components)
    }

    fn get_dir_from_components(&self, components: &[String]) -> Result<&Directory, FsError> {
        let mut current = &self.root;
        for component in components {
            current = match current.entries.get(component) {
                Some(Entry::Directory(dir)) => &**dir,
                Some(_) => return Err(FsError::NotADirectory),
                None => return Err(FsError::EntryNotFound),
            }
        }
        Ok(current)
    }

    fn get_dir_mut_from_components(&mut self, components: &[String]) -> Result<&mut Directory, FsError> {
        let mut current = &mut self.root;
        for component in components {
            current = match current.entries.get_mut(component) {
                Some(Entry::Directory(dir)) => &mut **dir,
                Some(_) => return Err(FsError::NotADirectory),
                None => return Err(FsError::EntryNotFound),
            }
        }
        Ok(current)
    }

    fn get_file(&self, path: &str) -> Result<&File, FsError> {
        let (dir_components, file_name) = Self::split_path(path)?;
        let dir = self.get_dir_from_components(&dir_components)?;
        
        match dir.entries.get(&file_name) {
            Some(Entry::File(file)) => Ok(&file),
            Some(_) => Err(FsError::NotAFile),
            None => Err(FsError::EntryNotFound),
        }
    }

    fn get_file_mut(&mut self, path: &str) -> Result<&mut File, FsError> {
        let (dir_components, file_name) = Self::split_path(path)?;
        let dir = self.get_dir_mut_from_components(&dir_components)?;
        
        match dir.entries.get_mut(&file_name) {
            Some(Entry::File(file)) => Ok(&mut *file),
            Some(_) => Err(FsError::NotAFile),
            None => Err(FsError::EntryNotFound),
        }
    }

    pub fn list_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        let dir = self.get_dir(path)?;
        Ok(dir.entries.keys().cloned().collect())
    }

    pub fn is_dir(&self, path: &str) -> bool {
        let components = match self.resolve_path(path) {
            Ok(c) => c,
            Err(_) => return false,
        };
        
        // Special case for root directory
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
}