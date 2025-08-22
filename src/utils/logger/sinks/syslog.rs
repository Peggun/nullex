use alloc::boxed::Box;

use crate::{
	fs::{self, ramfs::Permission},
	utils::logger::{
		levels::LogLevel,
		traits::{log_formatter::LogFormatter, logger_sink::LoggerSink}
	}
};

pub struct SyslogSink {
	pub formatter: Box<dyn LogFormatter>
}

impl SyslogSink {
	pub fn new(formatter: Box<dyn LogFormatter>) -> Self {
		Self {
			formatter
		}
	}
}

impl LoggerSink for SyslogSink {
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
