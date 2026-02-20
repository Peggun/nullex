//!
//! ports.rs
//! 
//! Port accessibility module for the kernel.
//! 

use core::arch::asm;

/// Write an 8-bit value to an I/O port (x86).
///
/// # Safety
/// - Uses inline `asm!` and performs port-mapped I/O; only valid on x86/x86_64.
/// - Caller must ensure the port is accessible and that performing the write is allowed
///   (privilege level, device readiness, etc.). Undefined behaviour may occur otherwise.
/// - This function is `unsafe` because it performs hardware I/O with side effects.
///
/// # Parameters
/// - `port`: 16-bit port address.
/// - `val`: 8-bit value to write.
///
/// # Notes
/// - Marked `#[inline(always)]` to encourage inlining for low-level code.
#[inline(always)]
pub unsafe fn outb(port: u16, val: u8) {
	unsafe {
		asm!(
			"out dx, al",
			in("dx") port,
			in("al") val,
			options(nomem, nostack, preserves_flags),
		);
	}
}

/// Read an 8-bit value from an I/O port (x86).
///
/// # Safety
/// - Uses inline `asm!` and performs port-mapped I/O; only valid on x86/x86_64.
/// - Caller must ensure the port is readable and that reading it is permitted in the current
///   execution context (privilege level, device state).
///
/// # Parameters
/// - `port`: 16-bit port address.
///
/// # Returns
/// - The `u8` value read from the port.
#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
	unsafe {
		let mut ret: u8;
		asm!(
			"in al, dx",
			in("dx") port,
			out("al") ret,
			options(nomem, nostack, preserves_flags),
		);
		ret
	}
}

/// Write a 16-bit value to an I/O port (x86).
///
/// # Safety
/// - Same safety considerations as `outb`.
///
/// # Parameters
/// - `port`: 16-bit port address.
/// - `val`: 16-bit value to write.
#[inline(always)]
pub unsafe fn outw(port: u16, val: u16) {
	unsafe {
		asm!(
			"out dx, ax",
			in("dx") port,
			in("ax") val,
			options(nomem, nostack, preserves_flags),
		);
	}
}

/// Read a 16-bit value from an I/O port (x86).
///
/// # Safety
/// - Same safety considerations as `inb`.
///
/// # Parameters
/// - `port`: 16-bit port address.
///
/// # Returns
/// - The `u16` value read from the port.
#[inline(always)]
pub unsafe fn inw(port: u16) -> u16 {
	unsafe {
		let mut ret: u16;
		asm!(
			"in ax, dx",
			in("dx") port,
			out("ax") ret,
			options(nomem, nostack, preserves_flags),
		);
		ret
	}
}

/// Write a 32-bit value to an I/O port (x86).
///
/// # Safety
/// - Same safety considerations as `outb`.
///
/// # Parameters
/// - `port`: 16-bit port address.
/// - `val`: 32-bit value to write.
#[inline(always)]
pub unsafe fn outl(port: u16, val: u32) {
	unsafe {
		asm!(
			"out dx, eax",
			in("dx") port,
			in("eax") val,
			options(nomem, nostack, preserves_flags),
		);
	}
}

/// Read a 32-bit value from an I/O port (x86).
///
/// # Safety
/// - Same safety considerations as `inb`.
///
/// # Parameters
/// - `port`: 16-bit port address.
///
/// # Returns
/// - The `u32` value read from the port.
#[inline(always)]
pub unsafe fn inl(port: u16) -> u32 {
	unsafe {
		let mut ret: u32;
		asm!(
			"in eax, dx",
			in("dx") port,
			out("eax") ret,
			options(nomem, nostack, preserves_flags),
		);
		ret
	}
}

/// Write a 64-bit value to two consecutive I/O ports (low dword first).
///
/// # Safety
/// - Performs two `outl` calls: writes the low 32 bits to `port` and the high 32 bits to `port + 4`.
/// - Not atomic: devices that expect an atomic 64-bit write may be left in an intermediate state.
/// - Caller must ensure ordering and that writing two dwords to consecutive ports is the correct
///   protocol for the target device.
///
/// # Parameters
/// - `port`: 16-bit base port address.
/// - `val`: 64-bit value to write.
#[inline(always)]
pub unsafe fn outq(port: u16, val: u64) {
	unsafe {
		outl(port, val as u32);
		outl(port.wrapping_add(4), (val >> 32) as u32);
	}
}

/// Read a 64-bit value from two consecutive I/O ports (low dword first).
///
/// # Safety
/// - Performs two `inl` calls: reads low 32 bits from `port` and high 32 bits from `port + 4`.
/// - Not atomic: the value may change between the two reads; caller must handle this if atomicity
///   is required (e.g., by device-specific locking or repeated-read checks).
/// - Caller must ensure the device supports this layout (low dword at `port`, high dword at `port+4`).
///
/// # Parameters
/// - `port`: 16-bit base port address.
///
/// # Returns
/// - The combined `u64` value constructed as `(high << 32) | low`.
#[inline(always)]
pub unsafe fn inq(port: u16) -> u64 {
	unsafe {
		let low = inl(port) as u64;
		let high = inl(port.wrapping_add(4)) as u64;
		(high << 32) | low
	}
}

/// Waits for an I/O operation to complete.
pub unsafe fn io_wait() {
	unsafe { outb(0x80, 0) };
}
