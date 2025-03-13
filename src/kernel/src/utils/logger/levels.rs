/// Defines the log level at which to log at
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LogLevel {
	Debug,
	Info,
	Warn,
	Error,
	Fatal
}
