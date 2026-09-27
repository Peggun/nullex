//!
//! time.rs
//!
//! Time handling module for the kernel.

use smoltcp::time::Instant;

/// Show the difference in time from the start to now.
pub fn elapsed_ms(start: Instant, now: Instant) -> i64 {
	now.total_millis().saturating_sub(start.total_millis())
}
