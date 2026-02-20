//!
//! levels.rs
//! 
//! Definitions for the different types of Logging Levels for the kernel's logging framework
//! 

#[derive(Debug, PartialEq, Clone, Copy)]
/// Enum representing all supported log levels.
pub enum LogLevel {
	/// Debug
	Debug,
	/// Information
	Info,
	/// Warnings
	Warn,
	/// Errors
	Error,
	/// Fatal (like kernel panics)
	Fatal
}
