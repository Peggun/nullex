//!
//! log_formatter.rs
//! 
//! Trait definitions for log formatters in the kernel's logging framework
//! 

use alloc::string::String;
use crate::utils::logger::levels::LogLevel;

/// Trait that all log formatters will need to implement.
pub trait LogFormatter: Send + Sync {
	/// Format the log message with a certain `LogLevel`
	fn format(&self, level: LogLevel, message: &str) -> String;
}
