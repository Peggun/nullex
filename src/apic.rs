// apic.rs

/*
APIC timer and register definitions.
*/

/*
APIC timer has two modes:
Divide-by-16 and Periodic Mode

Math Time!

Timer Frequency = Bus Frequency / Divide Value

Example:
If CPU Bus Frequency = 200,000,000 Hz (200 MHz)
Divide Value = 16

Timer Frequency = 200,000,000 Hz / 16
				= 12,500,000 Hz (12.5 MHz)

Ticks per millisecond = Timer Frequency / 1,000
					  = 12,500,000 Hz / 1,000
					  = 12,500 ticks/ms

Duration from ticks:
For 5 ms:
	Total Ticks = 5 ms * 12,500 ticks/ms
				= 62,500 ticks

Time from ticks:
For 62,500 ticks:
	Duration (ms) = 62,500 ticks / 12,500 ticks/ms
				  = 5 ms
*/

use core::sync::atomic::{AtomicU32, AtomicU64};

pub static TICKS_PER_MS: AtomicU64 = AtomicU64::new(6125);
pub static TICK_COUNT: AtomicU32 = AtomicU32::new(0);

/// APIC timer and register definitions.
pub mod apic {
	use core::{
		arch::x86_64::_rdtsc,
		ptr::{read_volatile, write_volatile},
		sync::atomic::{AtomicUsize, Ordering}
	};

	use x86_64::registers::model_specific::Msr;

	use super::{TICK_COUNT, TICKS_PER_MS};
	use crate::{
		apic::HumanReadableTime,
		compute_ticks_per_ms_from_sample,
		println,
		serial_println,
		task::yield_now,
		utils::cpu_utils::get_cpu_clock
	};

	/// The base address of the Local APIC (xAPIC mode).
	pub const APIC_BASE: usize = 0xFEE00000;

	// Register offsets (relative to the APIC base)
	pub const ID: usize = 0x020;
	pub const EOI: usize = 0x0B0;
	pub const SVR: usize = 0x0F0;
	pub const LVT_MASK: usize = 1 << 16;
	pub const LVT_TIMER: usize = 0x320;
	pub const TIMER_INIT_COUNT: usize = 0x380;
	pub const TIMER_CURRENT_COUNT: usize = 0x390;
	pub const TIMER_DIVIDE: usize = 0x3E0;

	// Timer mode and configuration bits.
	/// Bit flag for periodic mode in the LVT Timer Register.
	pub const TIMER_PERIODIC: u32 = 1 << 17;

	pub const TIMER_ONE_SHOT: u32 = 0 << 17;
	/// The interrupt vector you choose for timer interrupts (commonly 0x20).
	pub const TIMER_INTERRUPT_VECTOR: u32 = 0x20;
	/// Divide configuration value: here, 0x3 typically means divide by 16.
	pub const DIVIDE_BY_16: u32 = 0x3;

	/// How many milliseconds for a tick.
	pub const PERIOD_MS: u32 = 1;

	pub static DEBUG_ISR_FIRED: AtomicUsize = AtomicUsize::new(0);

	/// Write a 32-bit value to a Local APIC register.
	pub unsafe fn write_register(offset: usize, value: u32) {
		let reg = (APIC_BASE + offset) as *mut u32;
		unsafe { write_volatile(reg, value) };
	}

	/// Read a 32-bit value from a Local APIC register.
	pub unsafe fn read_register(offset: usize) -> u32 {
		let reg = (APIC_BASE + offset) as *const u32;
		unsafe { read_volatile(reg) }
	}

	/// Initialize and calibrate the APIC timer in periodic mode.
	pub unsafe fn init_timer() {
		println!("[Info] Initializing APIC Timer...");
		unsafe {
			let cpu_hz = get_cpu_clock() as u64;
			if cpu_hz == 0 || cpu_hz == u64::MAX {
				println!(
					"[Error] CPU clock unknown or invalid ({}). Aborting calibration.",
					cpu_hz
				);
				return;
			}

			write_register(TIMER_DIVIDE, DIVIDE_BY_16); // Set the timer divide configuration to divide by 16.
			write_register(
				LVT_TIMER,
				(LVT_MASK as u32 | TIMER_ONE_SHOT | (TIMER_INTERRUPT_VECTOR as u32))
					.try_into()
					.unwrap()
			);
			write_register(TIMER_INIT_COUNT, 0xFFFFFFFF);

			// calibrate
			let t0 = _rdtsc();

			// wait 100ms
			let measurement_ms = 100;
			let target_tsc: u64 = cpu_hz / 1000 * measurement_ms;
			loop {
				let now = _rdtsc();

				if now.wrapping_sub(t0) >= target_tsc {
					break;
				}

				core::hint::spin_loop();
			}

			let t1 = _rdtsc();

			let apic_now = read_register(TIMER_CURRENT_COUNT);
			let consumed = 0xFFFF_FFFF - apic_now;
			let elapsed_tsc = t1 - t0;

			serial_println!(
				"[Debug] t0={} t1={} elapsed_tsc={} consumed={}",
				t0,
				t1,
				elapsed_tsc,
				consumed
			);

			let ticks_per_ms =
				compute_ticks_per_ms_from_sample(consumed as u64, elapsed_tsc, cpu_hz).unwrap();
			if ticks_per_ms == 0 || ticks_per_ms == u64::MAX {
				serial_println!(
					"[Error] Ticks Per Millisecond unknown or invalid ({}). Aborting calibration.",
					ticks_per_ms
				);
			}

			TICKS_PER_MS.store(ticks_per_ms, Ordering::SeqCst);
			serial_println!("[Info] Calibrated ticks/ms = {}", ticks_per_ms);

			let initial_128 = (ticks_per_ms as u128).checked_mul(PERIOD_MS as u128);
			let initial_u32 = match initial_128 {
				Some(v) if v > 0 && v <= (u32::MAX as u128) => v as u32,
				Some(v) if v > (u32::MAX as u128) => {
					serial_println!(
						"[Error] Requested period too large for current ticks_per_ms (would overflow u32)."
					);
					write_register(
						LVT_TIMER,
						(LVT_MASK | TIMER_INTERRUPT_VECTOR as usize)
							.try_into()
							.unwrap()
					);
					return;
				}
				_ => {
					serial_println!("[Error] Invalid initial count computation.");
					write_register(
						LVT_TIMER,
						(LVT_MASK | TIMER_INTERRUPT_VECTOR as usize)
							.try_into()
							.unwrap()
					);
					return;
				}
			};

			// put it back to periodic after calibrating
			write_register(
				LVT_TIMER,
				(LVT_MASK | (TIMER_INTERRUPT_VECTOR as usize))
					.try_into()
					.unwrap()
			);
			write_register(TIMER_INIT_COUNT, initial_u32);
			write_register(LVT_TIMER, TIMER_PERIODIC | TIMER_INTERRUPT_VECTOR);

			serial_println!(
				"[Info] APIC timer set to periodic, period_ms = {}, initial = {}",
				PERIOD_MS,
				initial_u32
			);
		}

		println!("[Info] Done.")
	}

	/// Signal End-of-Interrupt (EOI) to the Local APIC.
	pub unsafe fn send_eoi() {
		unsafe { write_register(EOI, 0) };
	}

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
	/// and busy-waits. Make sure this is acceptable in your system context
	/// (e.g., if the APIC timer is also used for system ticks, interfering
	/// with it might cause timing issues).
	///
	/// `ticks_per_ms` is a calibrated value that indicates how many timer ticks
	/// correspond to one millisecond.
	pub async unsafe fn sleep(duration_ms: u32) {
		let start_tick = TICK_COUNT.load(Ordering::Acquire);
		let target_tick = start_tick + (duration_ms); // 1/2 tick = 500ms;
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
		(ticks as f32) / (TICKS_PER_MS.load(Ordering::Relaxed) as f32)
	}

	pub fn to_secs(ticks: u32) -> f32 {
		if ticks == 0 {
			return 0.0
		}
		(ticks as f32) / (TICKS_PER_MS.load(Ordering::Relaxed) as f32 * 1000.0)
	}

	pub fn to_mins(ticks: u32) -> f32 {
		if ticks == 0 {
			return 0.0
		}
		(ticks as f32) / (TICKS_PER_MS.load(Ordering::Relaxed) as f32 * 1000.0 * 60.0)
	}

	pub fn to_ticks(ms: u32) -> f32 {
		if ms == 0 {
			return 0.0
		}
		(ms as f32) * (TICKS_PER_MS.load(Ordering::Relaxed) as f32)
	}

	pub fn to_hrt(ticks: u32) -> HumanReadableTime {
		let total_ms = (ticks as u64 * 1000u64) / TICKS_PER_MS.load(Ordering::Relaxed);

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
}

#[derive(Clone, Copy, Debug)]
pub struct HumanReadableTime {
	pub days: u32,
	pub hours: u32,
	pub mins: u32,
	pub secs: u32,
	pub ms: u32
}
