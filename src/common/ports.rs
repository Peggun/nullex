// ports.rs
use core::arch::asm;

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

#[inline(always)]
pub unsafe fn outq(port: u16, val: u64) {
	unsafe {
		outl(port, val as u32);
		outl(port.wrapping_add(4), (val >> 32) as u32);
	}
}

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
