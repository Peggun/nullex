//!
//! utils/httparse/writer.rs
//!
//!

use alloc::{string::String, vec::Vec};

use crate::{error::NullexError, fs, fs::ramfs::Permission};

/// Sink that receives downloaded bytes.
pub trait DownloadedFileWriter {
	/// Receive the next chunk of downloaded data.
	fn write(&mut self, data: &[u8]) -> Result<(), NullexError>;

	/// Called when a download is complete and was successful.
	/// Implementations should flush here.
	fn finish(&mut self) -> Result<(), NullexError>;

	/// Called when the download is aborted due to an error.
	/// Implementations should discard partial data here.
	/// Default: do nothing.
	fn abort(&mut self) {}
}

/// Vector implementation of `DownloadedFileWriter`
pub struct VecDownloadedFileWriter {
	buf: Vec<u8>
}

#[allow(dead_code)]
impl VecDownloadedFileWriter {
	fn new() -> VecDownloadedFileWriter {
		VecDownloadedFileWriter {
			buf: Vec::new()
		}
	}

	fn with_capacity(cap: usize) -> VecDownloadedFileWriter {
		VecDownloadedFileWriter {
			buf: Vec::with_capacity(cap)
		}
	}
}

impl DownloadedFileWriter for VecDownloadedFileWriter {
	fn write(&mut self, data: &[u8]) -> Result<(), NullexError> {
		self.buf.extend_from_slice(data);
		Ok(())
	}

	fn finish(&mut self) -> Result<(), NullexError> {
		Ok(())
	}

	fn abort(&mut self) {
		self.buf.clear();
	}
}

/// Sink for writing downloaded files to the file system.
pub struct FileSystemDownloadedFileWriter {
	path: String,
	bytes_written: usize,
	finished: bool
}

impl FileSystemDownloadedFileWriter {
	/// Create a writer that will save to `path` in the ramfs.
	pub fn create(path: &str) -> Result<Self, NullexError> {
		Self::create_inner(path)
	}

	/// Like `create`; the capacity hint is ignored for chunked storage.
	pub fn create_with_capacity(path: &str, _capacity: usize) -> Result<Self, NullexError> {
		Self::create_inner(path)
	}

	fn create_inner(path: &str) -> Result<Self, NullexError> {
		let path = String::from(path);
		fs::with_fs(|fs| {
			if fs.exists(&path) {
				fs.remove(&path, false, false)
					.map_err(|_| NullexError::FsWriteError)?;
			}

			fs.create_chunked_file(&path, Permission::all())
				.map_err(|_| NullexError::FsWriteError)
		})?;

		Ok(Self {
			path,
			bytes_written: 0,
			finished: false
		})
	}

	/// How many bytes have been written
	pub fn bytes_written(&self) -> usize {
		self.bytes_written
	}
}

impl DownloadedFileWriter for FileSystemDownloadedFileWriter {
	fn write(&mut self, data: &[u8]) -> Result<(), NullexError> {
		fs::with_fs(|fs| {
			fs.write_file_chunked(&self.path, data)
				.map_err(|_| NullexError::FsWriteError)
		})?;
		self.bytes_written += data.len();
		Ok(())
	}

	fn finish(&mut self) -> Result<(), NullexError> {
		self.finished = true;
		Ok(())
	}

	fn abort(&mut self) {
		if self.finished {
			return;
		}

		let path = self.path.clone();
		fs::with_fs(|fs| {
			let _ = fs.remove(&path, false, false);
		});
	}
}
