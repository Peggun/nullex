// apic.rs

/*
APIC timer and register definitions.
*/

/*
APIC timer has two modes:
Divide-by-16, and Periodic Mode

Math Time!

Timer Frequency = Bus Frequency / Divide Value

For example, if we assume a bus frequency of 200MHz:
	Timer Frequency = 200,000,000 Hz / 16 = 12,500,000 Hz (12.5MHz)
	Ticks per millisecond = 12,500,000 Hz / 1000 = 12500 ticks/ms

Duration:
	5ms:
		Total Ticks = 5 ms * 12500 ticks/ms = 62500 ticks

Time from ticks:
	5ms:
		Duration (ms) = 62500 ticks / 12500 ticks/ms = 5 ms
*/

/// Read the math in apic.rs to understand the math behind this constant.
/// Note: When using CPUID to calibrate the timer, you might compute this value
/// at runtime.
pub const TICKS_PER_MS: u32 = 6125;

use core::sync::atomic::AtomicU32;

pub static TICK_COUNT: AtomicU32 = AtomicU32::new(0);

/// APIC timer and register definitions.
pub mod apic {
	use core::{
		ptr::{read_volatile, write_volatile},
		sync::atomic::Ordering
	};

	use x86_64::registers::model_specific::Msr;

	use super::{TICK_COUNT, TICKS_PER_MS};
	use crate::{
		errors::{KernelError, APIC_TIMER_CONFIGURATION_ERROR, APIC_TIMER_INIT_FAILED}, println, serial_println, task::yield_now
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
	/// The interrupt vector you choose for timer interrupts.
	pub const TIMER_INTERRUPT_VECTOR: u32 = 0x30;
	/// Divide configuration value: here, 0x3 typically means divide by 16.
	pub const DIVIDE_BY_16: u32 = 0x3;

	/// Write a 32-bit value to a Local APIC register.
	pub unsafe fn write_register(offset: usize, value: u32) {
		unsafe {
			let reg = (APIC_BASE + offset) as *mut u32;
			write_volatile(reg, value);
		}
	}

	/// Read a 32-bit value from a Local APIC register.
	pub unsafe fn read_register(offset: usize) -> u32 {
		unsafe {
			let reg = (APIC_BASE + offset) as *const u32;
			read_volatile(reg)
		}
	}

	/// Initialize the APIC timer in periodic mode.
	///
	/// `initial_count` is the value from which the timer will count down.
	/// You may need to calibrate this value based on your desired tick rate.
	///
	/// Returns Ok(()) on success or an appropriate KernelError.
	pub unsafe fn init_timer(initial_count: u32) -> Result<(), KernelError> {
		unsafe {
			serial_println!("[Info] Initializing APIC Timer...");
			if initial_count == 0 {
				serial_println!("[Error] APIC Timer initialization failed: initial_count is zero.");
				return Err(KernelError::ApicError(APIC_TIMER_INIT_FAILED));
			}
			write_register(TIMER_DIVIDE, DIVIDE_BY_16); // Divide by 16
			// Explicitly set periodic mode, vector 32, and unmask (bit 16 = 0)
			let lvt_config = TIMER_PERIODIC | TIMER_INTERRUPT_VECTOR; // Mask bit (16) is 0 by default
			write_register(LVT_TIMER, lvt_config);
			write_register(TIMER_INIT_COUNT, initial_count);
			let lvt_value = read_register(LVT_TIMER);
			if lvt_value != lvt_config {
				serial_println!("[Error] APIC Timer configuration error: expected {:#x}, got {:#x}", lvt_config, lvt_value);
				return Err(KernelError::ApicError(APIC_TIMER_CONFIGURATION_ERROR));
			}
			serial_println!("[Info] APIC Timer initialized successfully with count {}", initial_count);
			Ok(())
		}
	}

	/// Signal End-of-Interrupt (EOI) to the Local APIC.
	pub unsafe fn send_eoi() {
		unsafe {
			write_register(EOI, 0);
		}
	}

	/// Enable the APIC by setting the appropriate bit in the MSR.
	///
	/// Returns Ok(()) on success or an appropriate KernelError.
	pub unsafe fn enable_apic() -> Result<(), KernelError> {
		unsafe {
			println!("[Info] Enabling APIC Timer...");
			let mut msr = Msr::new(0x1B);
			let value = msr.read();
			println!("[Debug] APIC base MSR: {:#x}", value);
			let base_addr = value & 0xFFFFF000; // Mask to get physical base address
			if base_addr != APIC_BASE as u64 {
				panic!("[Error] APIC base mismatch: expected {:#x}, got {:#x}", APIC_BASE, base_addr);
			}
			msr.write(value | 0x800); // Set the "Enable APIC" bit (bit 11)

			// Verify that the APIC is enabled.
			let new_value = msr.read();
			if new_value & 0x800 == 0 {
				println!("[Error] Failed to enable APIC Timer: APIC enable bit not set.");
				return Err(KernelError::ApicError(APIC_TIMER_CONFIGURATION_ERROR));
			}

			println!("[Info] APIC Timer enabled successfully.");
			Ok(())
		}
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
	/// Returns Ok(()) after sleeping for the specified duration.
	pub async unsafe fn sleep(duration_ms: u32) -> Result<(), KernelError> {
		if duration_ms == 0 {
			return Ok(());
		}

		let start_tick = TICK_COUNT.load(Ordering::Acquire);
		let target_tick = start_tick + duration_ms; // target_tick in ms units

		while TICK_COUNT.load(Ordering::Acquire) < target_tick {
			yield_now().await;
		}

		Ok(())
	}

	/// Get the current tick count from the APIC timer.
	/// This is a monotonically increasing value that can be used for timing.
	/// The value is in timer ticks, which can be converted to milliseconds
	/// using the `TICKS_PER_MS` constant.
	pub fn now() -> u32 {
		TICK_COUNT.load(Ordering::Acquire)
	}

	/// Converts ticks to milliseconds.
	pub fn to_ms(ticks: u32) -> f32 {
		if ticks == 0 {
			return 0.0;
		}
		(ticks as f32) / (TICKS_PER_MS as f32)
	}

	/// Converts ticks to seconds.
	pub fn to_secs(ticks: u32) -> f32 {
		if ticks == 0 {
			return 0.0;
		}
		(ticks as f32) / (TICKS_PER_MS as f32 * 1000.0)
	}

	/// Converts ticks to minutes.
	pub fn to_mins(ticks: u32) -> f32 {
		if ticks == 0 {
			return 0.0;
		}
		(ticks as f32) / (TICKS_PER_MS as f32 * 1000.0 * 60.0)
	}

	/// Converts milliseconds to ticks.
	pub fn to_ticks(ms: u32) -> f32 {
		if ms == 0 {
			return 0.0;
		}
		(ms as f32) * (TICKS_PER_MS as f32)
	}
}
