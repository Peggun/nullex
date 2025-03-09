extern crate alloc;

use alloc::string::String;

use crate::utils::logger::levels::LogLevel;

pub trait LogFormatter: Send + Sync {
	fn format(&self, level: LogLevel, message: &str) -> String;
}
