use alloc::boxed::Box;

use lazy_static::lazy_static;

use crate::{
	apic,
	utils::logger::{
		format::DefaultFormatter,
		sinks::{stdout::StdOutSink, syslog::SyslogSink}
	}
};

pub static mut START_TICK: u32 = 0;
pub static mut START_TIME: f32 = 0.0;

pub fn initialize_constants() {
	unsafe {
		START_TICK = apic::apic::now();
		START_TIME = apic::apic::to_ms(START_TICK);
	}
}

lazy_static! {
	pub static ref STDOUT_SINK: StdOutSink =
		StdOutSink::new(Box::new(DefaultFormatter::new(true, true)));
	pub static ref SYSLOG_SINK: SyslogSink =
		SyslogSink::new(Box::new(DefaultFormatter::new(true, true)));
}
