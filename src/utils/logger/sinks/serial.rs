use log::{Record, Level, Metadata, Log};

use crate::serial_println;

pub struct SerialLogger;

impl Log for SerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace // captures everything
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            serial_println!("[{}] {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

