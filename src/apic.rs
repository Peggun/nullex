// apic.rs

/*
APIC timer and register definitions.
*/

/*
APIC timer has two modes
Divide-by-16, and Periodic Mode

Math Time!

Timer Frequency = Bus Frequency / Divide Value

Where we can change the Bus Frequency to CPU Speed, like 200MHz

Timer Frequency = 200,000,00 Hz (100MHz) / 16
= 12,500,000 Hz (12.5MHz)

Ticks per millisecond = 12,500,000 Hz / 1000 = 12500 ticks/ms

Duration:
	5ms:
		Total Ticks = 5 ms * 12500 ticks/ms = 62500 ticks

Time from ticks:
	5ms:
		Duration (ms) = 62500 ticks / 12500 ticks/ms = 5 ms
*/

/// Read the math in apic.rs to understand the math behind this constant.
pub const TICKS_PER_MS: u32 = 6125;

use core::sync::atomic::AtomicU32;

pub static TICK_COUNT: AtomicU32 = AtomicU32::new(0);

/// APIC timer and register definitions.
use core::{
	ptr::{read_volatile, write_volatile},
	sync::atomic::Ordering
};

/// The base address of the Local APIC (xAPIC mode).
pub const APIC_BASE: usize = 0xFEE00000;

// Register offsets (relative to the APIC base)
pub const ID: usize = 0x020;
pub const EOI: usize = 0x0B0;
pub const SVR: usize = 0x0F0;
pub const LVT_TIMER: usize = 0x320;
pub const TIMER_INIT_COUNT: usize = 0x380;
pub const TIMER_CURRENT_COUNT: usize = 0x390;
pub const TIMER_DIVIDE: usize = 0x3E0;

// Timer mode and configuration bits.
/// Bit flag for periodic mode in the LVT Timer Register.
pub const TIMER_PERIODIC: u32 = 0x20000;
/// The interrupt vector you choose for timer interrupts (commonly 0x20).
pub const TIMER_INTERRUPT_VECTOR: u32 = 0x20;
/// Divide configuration value: here, 0x3 typically means divide by 16.
pub const DIVIDE_BY_16: u32 = 0x3;

/// Write a 32-bit value to a Local APIC register.
/// # Safety
/// Volatile register write.
pub unsafe fn write_register(offset: usize, value: u32) {
	let reg = (APIC_BASE + offset) as *mut u32;
	unsafe { write_volatile(reg, value) };
}

/// Read a 32-bit value from a Local APIC register.
/// # Safety
/// Read register volatile.
pub unsafe fn read_register(offset: usize) -> u32 {
	let reg = (APIC_BASE + offset) as *const u32;
	unsafe { read_volatile(reg) }
}

/// Initialize the APIC timer in periodic mode.
// https://wiki.osdev.org/APIC_Timer#Example_code_in_C
// this code isnt great, i will came back later,
// maybe when the repo gets some attaction and people
// would like to help.
// like currently, for example, 5 seconds real-time
// is 0.6 seconds kernel time.
// fuck knows why. which is why i'll come back to it.
/// # Safety
/// Reads / Writes to registers volatile.
pub unsafe fn init_timer() {
	println!("[Info] Initializing APIC Timer...");
	unsafe {
		write_register(TIMER_DIVIDE, DIVIDE_BY_16); // Set the timer divide configuration to divide by 16.
		write_register(TIMER_INIT_COUNT, 0xFFFFFFFF);

		pit_sleep(1);

		write_register(LVT_TIMER, 1 << 16);

		let ticks_in_10ms = 0xFFFFFFFF - read_register(TIMER_CURRENT_COUNT);

		write_register(LVT_TIMER, 32 | TIMER_PERIODIC);
		write_register(TIMER_DIVIDE, DIVIDE_BY_16);
		write_register(TIMER_INIT_COUNT, ticks_in_10ms);
	}
	println!("[Info] Done.")
}

/// signal end of interrupt to the local apic.
/// # Safety
/// Volatile register writes.
pub unsafe fn send_eoi() {
	unsafe { write_register(EOI, 0) };
}

use x86_64::registers::model_specific::Msr;

use crate::{pit::pit_sleep, println, task::yield_now};

/// # Safety
/// Unsafe volatile reads.
pub unsafe fn enable_apic() {
	println!("[Info] Enabling APIC Timer...");
	let mut msr = Msr::new(0x1B);
	let value = unsafe { msr.read() };
	unsafe { msr.write(value | 0x800) }; // Set the "Enable APIC" bit (bit 11)
	println!("[Info] Done.");
}

/// Sleep for a given duration (in milliseconds) using the APIC timer.
///
/// # Safety
///
/// This function temporarily reconfigures the APIC timer to one-shot mode
/// and busy-waits.
///
/// `ticks_per_ms` is a calibrated value that indicates how many timer ticks
/// correspond to one millisecond. Not very well calibrated, but calibrated
pub async unsafe fn sleep(duration_ms: u32) {
	let start_tick = TICK_COUNT.load(Ordering::Acquire);
	let target_tick = start_tick + duration_ms; // 500 ticks = 500ms
	while TICK_COUNT.load(Ordering::Acquire) < target_tick {
		yield_now().await;
	}
}

/// Get the current tick count from the APIC timer.
/// This is a monotonically increasing value that can be used for timing.
/// The value is in timer ticks, which can be converted to milliseconds
/// using the `TICKS_PER_MS` constant.
pub fn now() -> u32 {
	TICK_COUNT.load(Ordering::Acquire)
}

pub fn to_ms(ticks: u32) -> f32 {
	if ticks == 0 {
		return 0.0
	}

	(ticks as f32) / (TICKS_PER_MS as f32)
}

pub fn to_secs(ticks: u32) -> f32 {
	if ticks == 0 {
		return 0.0
	}

	(ticks as f32) / (TICKS_PER_MS as f32 * 1000.0)
}

pub fn to_mins(ticks: u32) -> f32 {
	if ticks == 0 {
		return 0.0
	}

	(ticks as f32) / (TICKS_PER_MS as f32 * 1000.0 * 60.0)
}

pub fn to_ticks(ms: u32) -> f32 {
	if ms == 0 {
		return 0.0
	}

	(ms as f32) * (TICKS_PER_MS as f32)
}

/// Converts ticks into HumanReadableTime.
pub fn to_hrt(ticks: u32) -> HumanReadableTime {
	let total_ms = (ticks as u64 * 1000u64) / TICKS_PER_MS as u64;

	const MS_PER_DAY: u64 = 86_400_000; // 1000*60*60*24
	const MS_PER_HOUR: u64 = 3_600_000; // 1000*60*60
	const MS_PER_MIN: u64 = 60_000; // 1000*60
	const MS_PER_SEC: u64 = 1000;

	let days = (total_ms / MS_PER_DAY) as u32;
	let hours = ((total_ms % MS_PER_DAY) / MS_PER_HOUR) as u32;
	let mins = ((total_ms % MS_PER_HOUR) / MS_PER_MIN) as u32;
	let secs = ((total_ms % MS_PER_MIN) / MS_PER_SEC) as u32;
	let ms = (total_ms % MS_PER_SEC) as u32;

	HumanReadableTime {
		days,
		hours,
		mins,
		secs,
		ms
	}
}

#[derive(Clone, Copy, Debug)]
pub struct HumanReadableTime {
	pub days: u32,
	pub hours: u32,
	pub mins: u32,
	pub secs: u32,
	pub ms: u32
}
