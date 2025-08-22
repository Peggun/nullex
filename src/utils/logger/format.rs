use alloc::string::String;

use super::{levels::LogLevel, traits::log_formatter::LogFormatter};

pub struct DefaultFormatter {
	pub show_level: bool //pub show_timestamp: bool,
}

impl DefaultFormatter {
	pub fn new(show_level: bool) -> Self {
		Self {
			show_level
		}
	}
}

impl LogFormatter for DefaultFormatter {
	fn format(&self, level: LogLevel, message: &str) -> String {
		let mut formatted_message = String::new();
		if self.show_level {
			formatted_message.push_str(&format!("[{:#?}] ", level));
		}
		formatted_message.push_str(message);
		formatted_message
	}
}
