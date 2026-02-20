//! apic.rs

//!
//! APIC timer and register definitions.
//!

use core::{
	ptr::{read_volatile, write_volatile},
	sync::atomic::AtomicU64
};

use x86_64::instructions::interrupts;

use crate::{
	interrupts::APIC_TIMER_VECTOR, rtc::read_rtc_time, utils::mutex::SpinMutex
};

/// The base address of the APIC Timer
pub static APIC_BASE: SpinMutex<usize> = SpinMutex::new(0);
/// The number of times the APIC Timer has sent an interrupt
pub static APIC_TICK_COUNT: AtomicU64 = AtomicU64::new(0);
/// The TPS (Ticks per second) at which the APIC runs at
pub static APIC_TPS: AtomicU64 = AtomicU64::new(0);

// pic code

// pic ports
/// The Data Input port for PIC1
pub const PIC1_DATA: u16 = 0x21;
/// The Data Input port for PIC2
pub const PIC2_DATA: u16 = 0xA1;

// eoi
/// The Command Port for PIC1
pub const PIC1_CMD: u16 = 0x20;
/// The Command Port for PIC2
pub const PIC2_CMD: u16 = 0xA0;
/// The value for sending PIC an end of interrupt (EOI) signal
pub const PIC_EOI: u8 = 0x20;

// apic register offsets
// https://wiki.osdev.org/APIC
/// The ID for the APIC
pub const APIC_ID: usize = 0x020;
#[allow(unused)]
const APIC_VERSION: usize = 0x030;
#[allow(unused)]
const APIC_TPR: usize = 0x080;
const APIC_EOI: usize = 0x0B0;
const APIC_SVR: usize = 0x0F0;
#[allow(unused)]
const APIC_ISR_BASE: usize = 0x100; // ISR 0x100..0x170
#[allow(unused)]
const APIC_ICRLO: usize = 0x300;
#[allow(unused)]
const APIC_ICRHI: usize = 0x310;
const APIC_LVT_TIMER: usize = 0x320;
#[allow(unused)]
const APIC_LVT_THERMAL: usize = 0x330;
#[allow(unused)]
const APIC_LVT_PERF: usize = 0x340;
#[allow(unused)]
const APIC_LVT_LINT0: usize = 0x350;
#[allow(unused)]
const APIC_LVT_LINT1: usize = 0x360;
#[allow(unused)]
const APIC_LVT_ERROR: usize = 0x370;
const APIC_INITIAL_COUNT: usize = 0x380;
const APIC_CURRENT_COUNT: usize = 0x390;
const APIC_DIVIDE_CONF: usize = 0x3E0;

// bits/flags
const SVR_APIC_ENABLE: u32 = 1 << 8;
const LVT_MASK_BIT: u32 = 1 << 16;
const LVT_MODE_PERIODIC: u32 = 1 << 17;

#[inline(always)]
unsafe fn apic_reg_ptr(offset: usize) -> *mut u32 {
	let base = *APIC_BASE.lock();
	// Verify APIC_BASE is initialized (must be within APIC memory range)
	if base == 0 || base < 0xFED0_0000 {
		panic!("APIC_BASE not initialized or invalid: {:#x}", base);
	}
	(base + offset) as *mut u32
}

#[inline(always)]
/// Read APIC register.
pub unsafe fn read_register(offset: usize) -> u32 {
	unsafe {
		let p = apic_reg_ptr(offset);
		read_volatile(p)
	}
}

#[inline(always)]
/// Write APIC register.
unsafe fn write_register(offset: usize, val: u32) {
	unsafe {
		let p = apic_reg_ptr(offset);
		write_volatile(p, val);

		// apic usually needs a read after writing
		let _ = read_volatile(apic_reg_ptr(APIC_ID));
	}
}

/// Enables APIC by setting the Spurious Vector Bit to enabled.
pub unsafe fn enable_apic(spurious_vector: u8) {
	unsafe {
		let mut svr = (spurious_vector as u32) & 0xFF;
		svr |= SVR_APIC_ENABLE;
		write_register(APIC_SVR, svr);
	}
}

/// Write EOI to the Local APIC and acknowledge the interrupt.
pub unsafe fn send_eoi() {
	unsafe {
		write_register(APIC_EOI, 0);
	}
}

/// Set the timer divide configuration.
unsafe fn set_timer_divide(divide_cfg: u32) {
	unsafe {
		write_register(APIC_DIVIDE_CONF, divide_cfg & 0xF);
	}
}

/// Set the APIC timer initial count (TICR)
unsafe fn set_timer_initial(count: u32) {
	unsafe {
		write_register(APIC_INITIAL_COUNT, count);
	}
}

/// Read current count (TCCR)
unsafe fn read_current_count() -> u32 {
	unsafe { read_register(APIC_CURRENT_COUNT) }
}

/// Configure the LVT timer
unsafe fn configure_lvt_timer(vector: u8, periodic: bool, masked: bool) {
	unsafe {
		let mut entry = (vector as u32) & 0xFF;
		if periodic {
			entry |= LVT_MODE_PERIODIC;
		}
		if masked {
			entry |= LVT_MASK_BIT;
		}
		write_register(APIC_LVT_TIMER, entry);
	}
}

/// Mask / unmask the timer interrupt.
pub unsafe fn mask_timer(mask: bool) {
	unsafe {
		let mut r = read_register(APIC_LVT_TIMER);
		if mask {
			r |= LVT_MASK_BIT;
		} else {
			r &= !LVT_MASK_BIT;
		}
		write_register(APIC_LVT_TIMER, r);
	}
}

/// Set the LVT timer into periodic mode with a initial count.
pub unsafe fn start_timer_periodic(timer_vector: u8, initial_count: u32) {
	unsafe {
		set_timer_divide(0x3);
		set_timer_initial(initial_count);
		configure_lvt_timer(timer_vector, true, false);
	}
}

/// Calibrate the LAPIC timer using the RTC
///
/// Returns the (ticks_per_second, recommended_initial_count) on success
pub fn calibrate(target_hz: u32) -> Result<(u64, u32), &'static str> {
	if target_hz == 0 {
		return Err("target_hz must be > 0")
	}

	interrupts::disable();

	unsafe {
		mask_timer(true);
		set_timer_divide(0x3);
		set_timer_initial(0xFFFF_FFFFu32);

		// Use masked = true while measuring (we don't want the hardware to interrupt
		// us). periodic = false (one-shot) is fine for measuring.
		configure_lvt_timer(APIC_TIMER_VECTOR, true, false);
	}

	// Read start counter while still masked
	let start_count = unsafe { read_current_count() };

	// Wait for RTC second tick — no interrupts required
	let s_before = read_rtc_time().sec;
	loop {
		let s_now = read_rtc_time().sec;
		if s_now != s_before {
			break;
		}
	}

	// Read end counter
	let end_count = unsafe { read_current_count() };

	let ticks_per_second = start_count.wrapping_sub(end_count) as u64;
	if ticks_per_second == 0 {
		return Err("measured zero ticks_per_second; calibration failed");
	}

	let initial_count_u64 = ticks_per_second / (target_hz as u64);
	if initial_count_u64 == 0 || initial_count_u64 > u32::MAX as u64 {
		return Err("computed invalid initial_count; adjust target_hz or check APIC timer range");
	}
	let initial_count = initial_count_u64 as u32;

	unsafe {
		mask_timer(true);
		configure_lvt_timer(APIC_TIMER_VECTOR, true, true); // keep masked; set periodic
	}

	// still safe to return while interrupts remain disabled
	Ok((ticks_per_second, initial_count))
}

/// Prelude module for APIC.
pub mod prelude {
	pub use crate::apic::*;
}
