use alloc::boxed::Box;

use crate::{
	fs::{self, ramfs::Permission},
	utils::logger::{
		levels::LogLevel,
		traits::{log_formatter::LogFormatter, logger_sink::LoggerSink}
	}
};

pub struct SyslogSink {
	pub formatter: Box<dyn LogFormatter> //pub config: LoggerConfig
}

impl SyslogSink {
	/// Create a new `StdOutSink` with formatter.
	/// # Arguments
	/// * `formatter: Box<dyn LogFormatter>` - The formatter using for the
	///   logger
	///
	/// # Returns
	/// * New `StdOutSink` Instance
	pub fn new(formatter: Box<dyn LogFormatter>) -> Self {
		Self {
			formatter
		}
	}
}

impl LoggerSink for SyslogSink {
	/// The non-asynchronous log function.
	/// # Arguments
	/// * `message: &str` - The message to be logged.
	/// * `level: LogLevel` - The `LogLevel` to the log to be written.
	fn log(&self, message: &str, level: LogLevel) {
		let formatted_message = self.formatter.format(level, message);
		fs::with_fs(|fs| {
			if !fs.exists("/logs") {
				let _ = fs.create_dir("/logs", Permission::all());
			}
			if !fs.exists("/logs/syslog") {
				let _ = fs.create_file("/logs/syslog", Permission::all());
			}
			let _ = fs.write_file("/logs/syslog", formatted_message.as_bytes());
		})
	}

	/// The asynchronous log function.
	/// # Arguments
	/// * `message` - The message to be logged.
	/// * `level` - The `LogLevel` to the log to be written.
	///
	/// # Returns
	/// - `core::future::Future<Output = ()> + Send` (use .await;)
	fn log_async(
		&self,
		message: &str,
		level: LogLevel
	) -> impl core::future::Future<Output = ()> + Send {
		let formatted_message = self.formatter.format(level, message);
		async move {
			fs::with_fs(|fs| {
				if !fs.exists("/logs") {
					let _ = fs.create_dir("/logs", Permission::all());
				}
				if !fs.exists("/logs/syslog") {
					let _ = fs.create_file("/logs/syslog", Permission::all());
				}
				let _ = fs.write_file("/logs/syslog", formatted_message.as_bytes());
			})
		}
	}
}
