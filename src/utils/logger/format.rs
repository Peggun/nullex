use alloc::string::String;

use super::{levels::LogLevel, traits::log_formatter::LogFormatter};
use crate::{apic::apic, constants::START_TIME};

pub struct DefaultFormatter {
	pub show_level: bool,
	pub show_timestamp: bool
}

impl DefaultFormatter {
	pub fn new(show_level: bool, show_timestamp: bool) -> Self {
		Self {
			show_level,
			show_timestamp
		}
	}
}

impl LogFormatter for DefaultFormatter {
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
