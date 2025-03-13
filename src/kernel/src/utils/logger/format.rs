use alloc::string::String;

use super::{levels::LogLevel, traits::log_formatter::LogFormatter};
use crate::{apic::apic, constants::START_TIME};

/// The default formatter used for most logging tasks.
pub struct DefaultFormatter {
	pub show_level: bool,
	pub show_timestamp: bool
}

impl DefaultFormatter {
	/// Creates a new `DefaultFormatter` instance.
	/// # Arguments
	/// * `show_level: bool - Show the log level in the logs.
	/// * `show_timestamp: bool` - Show the timestamp in the logs.
	///
	/// # Returns
	/// * `DefaultFormatter` - The `DefaultFormatter` instance.
	pub fn new(show_level: bool, show_timestamp: bool) -> Self {
		Self {
			show_level,
			show_timestamp
		}
	}
}

impl LogFormatter for DefaultFormatter {
	/// Formats the log message according to Formatter Settings
	/// # Arguments
	/// * `level: LogLevel` - The `LogLevel` to log at
	/// * `message: &str` - The message to log
	///
	/// # Returns
	/// * `String` - The formatted message
	fn format(&self, level: LogLevel, message: &str) -> String {
		unsafe {
			let mut formatted_message = String::new();
			if self.show_level {
				formatted_message.push_str(&format!("[{:#?}] ", level));
			}
			if self.show_timestamp {
				let now = apic::to_ms(apic::now());
				let time = now - START_TIME;
				formatted_message.push_str(&format!("[{:#?}] ", time));
			}
			formatted_message.push_str(message);
			formatted_message
		}
	}
}
