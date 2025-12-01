// apic.rs

/*
APIC timer and register definitions.
*/

use core::{
	ptr::{read_volatile, write_volatile},
	sync::atomic::AtomicU64
};

use x86_64::instructions::interrupts;

use crate::{interrupts::APIC_TIMER_VECTOR, rtc::read_rtc_raw, utils::mutex::SpinMutex};

pub static APIC_BASE: SpinMutex<usize> = SpinMutex::new(0);
pub static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

// apic register offsets
pub const APIC_ID: usize = 0x020;
pub const APIC_VERSION: usize = 0x030;
pub const APIC_TPR: usize = 0x080;
pub const APIC_EOI: usize = 0x0B0;
pub const APIC_SVR: usize = 0x0F0;
pub const APIC_ISR_BASE: usize = 0x100; // ISR 0x100..0x170
pub const APIC_ICRLO: usize = 0x300;
pub const APIC_ICRHI: usize = 0x310;
pub const APIC_LVT_TIMER: usize = 0x320;
pub const APIC_LVT_THERMAL: usize = 0x330;
pub const APIC_LVT_PERF: usize = 0x340;
pub const APIC_LVT_LINT0: usize = 0x350;
pub const APIC_LVT_LINT1: usize = 0x360;
pub const APIC_LVT_ERROR: usize = 0x370;
pub const APIC_INITIAL_COUNT: usize = 0x380;
pub const APIC_CURRENT_COUNT: usize = 0x390;
pub const APIC_DIVIDE_CONF: usize = 0x3E0;

// bits/flags
pub const SVR_APIC_ENABLE: u32 = 1 << 8;
pub const LVT_MASK_BIT: u32 = 1 << 16;
pub const LVT_MODE_PERIODIC: u32 = 1 << 17;

#[inline(always)]
unsafe fn apic_reg_ptr(offset: usize) -> *mut u32 {
	let base = *APIC_BASE.lock();
	(base + offset) as *mut u32
}

#[inline(always)]
/// Read APIC register.
pub unsafe fn read_register(offset: usize) -> u32 {
	let p = apic_reg_ptr(offset);
	read_volatile(p)
}

#[inline(always)]
/// Write APIC register.
pub unsafe fn write_register(offset: usize, val: u32) {
	let p = apic_reg_ptr(offset);
	write_volatile(p, val);

	// apic usually needs a read after writing
	let _ = read_volatile(apic_reg_ptr(APIC_ID));
}

/// Enables APIC by setting the Spurious Vector Bit to enabled.
pub unsafe fn enable_apic(spurious_vector: u8) {
	let mut svr = (spurious_vector as u32) & 0xFF;
	svr |= SVR_APIC_ENABLE;
	write_register(APIC_SVR, svr);
}

/// Write EOI to the Local APIC and acknowledge the interrupt.
pub unsafe fn send_eoi() {
	write_register(APIC_EOI, 0);
}

/// Set the timer divide configuration.
pub unsafe fn set_timer_divide(divide_cfg: u32) {
	write_register(APIC_DIVIDE_CONF, divide_cfg & 0xF);
}

/// Set the APIC timer initial count (TICR)
pub unsafe fn set_timer_initial(count: u32) {
	write_register(APIC_INITIAL_COUNT, count);
}

/// Read current count (TCCR)
pub unsafe fn read_current_count() -> u32 {
	read_register(APIC_CURRENT_COUNT)
}

/// Initialises the APIC timer in a simple default. Used before calibrating.
pub unsafe fn init_timer_default(timer_vector: u8) {
	configure_lvt_timer(timer_vector, false, true);
	set_timer_divide(0x3);
	set_timer_initial(0xFFFF_FFFFu32);
	configure_lvt_timer(timer_vector, true, true);
}

/// Configure the LVT timer
pub unsafe fn configure_lvt_timer(vector: u8, periodic: bool, masked: bool) {
	let mut entry = (vector as u32) & 0xFF;
	if periodic {
		entry |= LVT_MODE_PERIODIC;
	}
	if masked {
		entry |= LVT_MASK_BIT;
	}
	write_register(APIC_LVT_TIMER, entry);
}

/// Mask / unmask the timer interrupt.
pub unsafe fn mask_timer(mask: bool) {
	let mut r = read_register(APIC_LVT_TIMER);
	if mask {
		r |= LVT_MASK_BIT;
	} else {
		r &= !LVT_MASK_BIT;
	}
	write_register(APIC_LVT_TIMER, r);
}

/// Set the LVT timer into periodic mode with a initial count.
pub unsafe fn start_timer_periodic(timer_vector: u8, initial_count: u32) {
	set_timer_divide(0x3);
	set_timer_initial(initial_count);
	configure_lvt_timer(timer_vector, true, false);
}

/// Set the LVT timer into one-shot mode with a initial count.
pub unsafe fn start_timer_one_shot(timer_vector: u8, initial_count: u32) {
	set_timer_divide(0x3);
	set_timer_initial(initial_count);
	configure_lvt_timer(timer_vector, false, false); // unmask one-shot
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
		configure_lvt_timer(APIC_TIMER_VECTOR, false, true);
	}

	let start_count = unsafe { read_current_count() };

	let (s_before, _m, _h, _d, _mo, _y) = read_rtc_raw();
	loop {
		let (s_now, _, _, _, _, _) = read_rtc_raw();
		if s_now != s_before {
			break;
		}
	}

	let end_count = unsafe { read_current_count() };

	interrupts::enable();

	let ticks_per_second = start_count.wrapping_sub(end_count) as u64;
	if ticks_per_second == 0 {
		return Err("measured zero ticks_per_second; calibration failed");
	}

	let initial_count_u64 = ticks_per_second / (target_hz as u64);
	if initial_count_u64 == 0 || initial_count_u64 > u32::MAX as u64 {
		return Err("computed invalid initial_count; adjust target_hz or check APIC timer range");
	}
	let initial_count = initial_count_u64 as u32;

	Ok((ticks_per_second, initial_count))
}

pub mod prelude {
	pub use crate::apic::*;
}
