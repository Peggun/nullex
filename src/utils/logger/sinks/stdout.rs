use alloc::boxed::Box;

use crate::{
	println,
	utils::logger::{
		levels::LogLevel,
		traits::{log_formatter::LogFormatter, logger_sink::LoggerSink}
	}
};

pub struct StdOutSink {
	pub formatter: Box<dyn LogFormatter>
}

impl StdOutSink {
	pub fn new(formatter: Box<dyn LogFormatter>) -> Self {
		Self {
			formatter
		}
	}
}

impl LoggerSink for StdOutSink {
	fn log(&self, message: &str, level: LogLevel) {
		let formatted_message = self.formatter.format(level, message);
		println!("{}", formatted_message);
	}

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
