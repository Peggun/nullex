//!
//! src/utils/logger/sinks/mod.rs 
//! 
//! All sink definitions for the kernel's logging framework
//! 

pub mod stdout;
pub mod syslog;

use alloc::boxed::Box;

use crate::{
	lazy_static,
	utils::logger::{
		format::DefaultFormatter,
		sinks::{stdout::StdOutSink, syslog::SyslogSink}
	}
};

lazy_static! {
	/// Static reference to the Standard Output Sink
	pub static ref STDOUT_SINK: StdOutSink = StdOutSink::new(Box::new(DefaultFormatter::new(true)));
	/// Static reference to the System Logging Sink
	pub static ref SYSLOG_SINK: SyslogSink = SyslogSink::new(Box::new(DefaultFormatter::new(true)));
}
