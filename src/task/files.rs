use alloc::string::String;

use crate::net::socket::SocketTransport;

/// Backend for a file.
#[derive(Debug)]
pub enum FileBackend {
	/// File on disk.
	DiskFile {
		/// File path
		path: String,
		/// File offset
		offset: usize
	},
	/// Directory on disk.
	Directory { path: String },
	/// Standard In file.
	Stdin,
	/// Standard out file.
	Stdout,
	/// Standard error File.
	Stderr,
	/// Socket file.
	Socket { connection: Option<SocketTransport> }
}

/// Struct to represent an open file in a process
#[derive(Debug)]
pub struct OpenFile {
	/// The backend of the open file.
	pub backend: FileBackend
}

impl OpenFile {
	pub fn path(&self) -> Option<&str> {
		match &self.backend {
			FileBackend::DiskFile {
				path, ..
			}
			| FileBackend::Directory {
				path
			} => Some(path),
			FileBackend::Stdin
			| FileBackend::Stdout
			| FileBackend::Stderr
			| FileBackend::Socket {
				..
			} => None
		}
	}
}
