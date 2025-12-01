use alloc::boxed::Box;

use crate::{
	apic,
	lazy_static,
	utils::logger::{
		format::DefaultFormatter,
		sinks::{stdout::StdOutSink, syslog::SyslogSink}
	}
};

lazy_static! {
	pub static ref STDOUT_SINK: StdOutSink = StdOutSink::new(Box::new(DefaultFormatter::new(true)));
	pub static ref SYSLOG_SINK: SyslogSink = SyslogSink::new(Box::new(DefaultFormatter::new(true)));
}
