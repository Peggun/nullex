//!
//! src/utils/logger/mod.rs
//!
//! Module definition for the logging framework for the kernel.

use crate::utils::logger::sinks::serial::SerialLogger;

pub mod format;
pub mod levels;
pub mod sinks;
pub mod traits;

pub static LOGGER: SerialLogger = SerialLogger;

pub fn init_logging() {
	log::set_logger(&LOGGER)
		.map(|()| log::set_max_level(log::LevelFilter::Trace))
		.unwrap();
}
