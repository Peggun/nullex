use alloc::boxed::Box;

use crate::{
	println,
	utils::logger::{
		levels::LogLevel,
		traits::{log_formatter::LogFormatter, logger_sink::LoggerSink}
	}
};

/// The Standard Output Sink (serial driver)
pub struct StdOutSink {
	pub formatter: Box<dyn LogFormatter>
}

impl StdOutSink {
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

impl LoggerSink for StdOutSink {
	/// The non-asynchronous log function.
	/// # Arguments
	/// * `message: &str` - The message to be logged.
	/// * `level: LogLevel` - The `LogLevel` to the log to be written.
	fn log(&self, message: &str, level: LogLevel) {
		let formatted_message = self.formatter.format(level, message);
		println!("{}", formatted_message);
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
			println!("{}", formatted_message);
		}
	}
}
