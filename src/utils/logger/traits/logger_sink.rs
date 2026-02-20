//!
//! logger_sink.rs
//! 
//! Trait definition for all types that are a sink.
//! 

use crate::utils::logger::levels::LogLevel;

/// Trait representing all functions that a logging sink will need to implement.
pub trait LoggerSink {
	/// Log a message of a certain level.
	fn log(&self, message: &str, level: LogLevel);
	/// Asynchronously Log a message of a certain level.
	fn log_async(
		&self,
		message: &str,
		level: LogLevel
	) -> impl core::future::Future<Output = ()> + Send;
}
