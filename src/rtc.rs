//! rtc.rs
//!
//! RTC (Real Time Clock) module for the kernel.

use alloc::string::String;
use core::{
	fmt,
	sync::atomic::{AtomicU64, Ordering}
};

use smoltcp::time::Instant;
use x86_64::instructions::interrupts;

use crate::{
	apic::send_eoi,
	common::ports::{inb, io_wait, outb},
	ioapic::IOAPIC,
	serial_println
};

/// The CMOS/RTC Index Register
pub const CMOS_INDEX: u16 = 0x70;
/// The CMOS/RTC Data Port
pub const CMOS_DATA: u16 = 0x71;

/// The CMOS/RTC NMI (Non-maskable interrupt) value.
pub const NMI_BIT: u8 = 0x80;

// regs
const REG_SECONDS: u8 = 0x00;
const REG_MINUTES: u8 = 0x02;
const REG_HOURS: u8 = 0x04;
const REG_DAY: u8 = 0x07;
const REG_MONTH: u8 = 0x08;
const REG_YEAR: u8 = 0x09;

/// Register A of the CMOS/RTC<br>
/// Controls the time update process and the square wave output
pub const REG_A: u8 = 0x0A;
/// Register B of the CMOS/RTC<br>
/// Controls the RTC's operating modes and interrupts
pub const REG_B: u8 = 0x0B;
/// **READ_ONLY**<br>
/// Register C of the CMOS/RTC<br>
/// Indicates which interrupt has occurred.
pub const REG_C: u8 = 0x0C;
/// **READ_ONLY**<br>
/// Register D of the CMOS/RTC<br>
/// Indicates battery status
pub const REG_D: u8 = 0x0D;

// rtc bits
const REG_A_UIP: u8 = 0x80;
const REG_B_PIE: u8 = 0x40;
const REG_B_DM: u8 = 0x04;

/// The number of times the RTC interrupt has gone off

pub static RTC_TICKS: AtomicU64 = AtomicU64::new(0);

#[derive(Copy, Clone)]
/// A structure representing the time which is returned by the RTC
pub struct RtcTime {
	/// The number of milliseconds
	pub millis: u8,
	/// The number of seconds
	pub sec: u8,
	/// The number of minutes
	pub min: u8,
	/// The number of hours
	pub hour: u8,
	/// The number of days
	pub day: u8,
	/// The number of months
	pub month: u8,
	/// The number of full years
	pub year: u16 // full year
}

impl fmt::Display for RtcTime {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut s = String::new();

		s.push_str(&format!("{}/{}/{}", self.day, self.month, self.year));

		if self.hour <= 9 {
			s.push_str(&format!(" 0{}:", self.hour));
		} else {
			s.push_str(&format!(" {}:", self.hour));
		}

		if self.min <= 9 {
			s.push_str(&format!("0{}:", self.min));
		} else {
			s.push_str(&format!("{}:", self.min));
		}

		if self.sec <= 9 {
			s.push_str(&format!("0{}", self.sec));
		} else {
			s.push_str(&format!("{}", self.sec));
		}

		write!(f, "{}", s)
	}
}

/// Get RTC tick count
pub fn rtc_ticks() -> u64 {
	RTC_TICKS.load(Ordering::Acquire)
}

#[inline]
fn bcd_to_bin(b: u8) -> u8 {
	(b & 0xF) + ((b / 16) * 10)
}

#[inline(always)]
/// Read a value from a CMOS register.
fn cmos_read(reg: u8) -> u8 {
	x86_64::instructions::interrupts::without_interrupts(|| unsafe {
		outb(CMOS_INDEX, reg);
		io_wait();
		let val = inb(CMOS_DATA);
		io_wait();
		val
	})
}

#[inline(always)]
/// Write a value into a CMOS register.
fn cmos_write(reg: u8, value: u8) {
	x86_64::instructions::interrupts::without_interrupts(|| unsafe {
		outb(CMOS_INDEX, reg);
		io_wait();
		outb(CMOS_DATA, value);
		io_wait();
	})
}

/// Unmask RTC GSI 8 on the IOAPIC.
pub fn unmask_rtc_gsi8() {
	unsafe {
		let mut ioapic = IOAPIC.lock();
		ioapic.enable_irq(8);
	}
}

/// Returns the (millis, secs, mins, hours, days, months, years) in the RTC
/// clock raw.
fn read_rtc_raw() -> (u8, u8, u8, u8, u8, u8, u8) {
	loop {
		// wait for any update in progress to finish
		while (cmos_read(REG_A) & REG_A_UIP) != 0 {}

		let s1 = cmos_read(REG_SECONDS);
		let m1 = cmos_read(REG_MINUTES);
		let h1 = cmos_read(REG_HOURS);
		let d1 = cmos_read(REG_DAY);
		let mo1 = cmos_read(REG_MONTH);
		let y1 = cmos_read(REG_YEAR);

		// ensure no update started during the second read
		while (cmos_read(REG_A) & REG_A_UIP) != 0 {}

		let s2 = cmos_read(REG_SECONDS);
		let m2 = cmos_read(REG_MINUTES);
		let h2 = cmos_read(REG_HOURS);
		let d2 = cmos_read(REG_DAY);
		let mo2 = cmos_read(REG_MONTH);
		let y2 = cmos_read(REG_YEAR);

		if s1 == s2 && m1 == m2 && h1 == h2 && d1 == d2 && mo1 == mo2 && y1 == y2 {
			let ms = rtc_millis();
			return (ms.try_into().unwrap(), s1, m1, h1, d1, mo1, y1);
		}
		// else try again
	}
}

fn rtc_millis() -> u64 {
	let ticks = rtc_ticks();
	((ticks % 1024) * 1000) / 1024
}

/// Read RTC values to calculate the time/calendar.
pub fn read_rtc_time() -> RtcTime {
	let reg_b = cmos_read(REG_B);
	let bin_mode = (reg_b & REG_B_DM) != 0; // binary_mode. needs bcd -> bin
	let is_24hr = (reg_b & 0x02) != 0;

	let (ms, s, m, h_raw, d, mo, y) = read_rtc_raw();

	let hour = if is_24hr {
		h_raw & 0x7F
	} else {
		// 12hr. high bit is PM (like AM and PM) flag
		let pm = (h_raw & 0x80) != 0;
		let mut h12 = h_raw & 0x7F;
		if h12 == 12 {
			// 12AM => 0 || 12 PM => 12
			if !pm {
				h12 = 0;
			}
		} else if pm {
			h12 = h12.wrapping_add(12);
		}

		h12
	};

	let sec = if bin_mode { s } else { bcd_to_bin(s) };
	let min = if bin_mode { m } else { bcd_to_bin(m) };
	let hour = if bin_mode { hour } else { bcd_to_bin(hour) };
	let day = if bin_mode { d } else { bcd_to_bin(d) };
	let month = if bin_mode { mo } else { bcd_to_bin(mo) };

	let year_full = if bin_mode {
		2000u16 + y as u16
	} else {
		2000u16 + bcd_to_bin(y) as u16
	};

	RtcTime {
		millis: ms,
		sec,
		min,
		hour,
		day,
		month,
		year: year_full
	}
}

/// Get the `RTC` time as a `Instant`
pub fn rtc_instant() -> Instant {
	let ms = rtc_ticks().saturating_mul(1000) / 1024;
	Instant::from_millis(ms as i64)
}

/// Initializes the Real Time Clock (RTC)
pub fn init_rtc() {
	let saved = interrupts::are_enabled();
	interrupts::disable();

	// set rate
	let prev_a = cmos_read(REG_A);
	cmos_write(REG_A, (prev_a & 0xF0) | 0x06); // rs = 6

	// enable PIE
	let prev_b = cmos_read(REG_B);
	cmos_write(REG_B, prev_b | REG_B_PIE | REG_B_DM);

	// clear pending interrupts
	let _ = cmos_read(REG_C);

	if saved {
		interrupts::enable();
	}
}

/// Send a End of Interrupt (EOI) signal to the CPU for the RTC.
pub unsafe fn send_rtc_eoi() {
	unsafe {
		send_eoi();
	} // use LAPIC/IOAPIC EOI
}

// debug
// maybe eventually compile with a debug feature?
fn _dump_rtc_and_pic_state() {
	serial_println!("--- RTC/CMOS dump ---");
	for r in 0x00..=0x0D {
		serial_println!("CMOS reg {:#04x} = {:#04x}", r, cmos_read(r));
	}

	let pic1_data: u16 = 0x21;
	let pic2_data: u16 = 0xA1;
	let m = unsafe { inb(pic1_data) };
	let s = unsafe { inb(pic2_data) };
	serial_println!("PIC1 mask = {:#04x}, PIC2 mask = {:#04x}", m, s);
	serial_println!("--- end dump ---");
}

/// Prelude module for all rtc items.
pub mod prelude {
	pub use crate::rtc::*;
}

#[cfg(feature = "test")]
pub mod tests {
	use crate::{rtc::prelude::*, utils::ktest::TestError};

	pub fn test_bcd_to_bin_examples() -> Result<(), TestError> {
		assert_eq!(bcd_to_bin(0x00), 0);
		assert_eq!(bcd_to_bin(0x12), 12);
		assert_eq!(bcd_to_bin(0x59), 59);
		Ok(())
	}
	crate::create_test!(test_bcd_to_bin_examples);

	pub fn test_rtc_ticks_atomic_accessors() -> Result<(), TestError> {
		RTC_TICKS.store(0xDEADBEEF, Ordering::Relaxed);
		assert_eq!(rtc_ticks(), 0xDEADBEEF);
		Ok(())
	}
	crate::create_test!(test_rtc_ticks_atomic_accessors);
}
