use crate::utils::logger::levels::LogLevel;

pub trait LoggerSink {
	fn log(&self, message: &str, level: LogLevel);
	fn log_async(
		&self,
		message: &str,
		level: LogLevel
	) -> impl core::future::Future<Output = ()> + Send;
}
