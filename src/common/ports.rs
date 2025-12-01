// ports.rs
use core::arch::asm;

#[inline(always)]
pub unsafe fn outb(port: u16, data: u8) {
	unsafe {
		asm!(
			"out dx, al",
			in("dx") port,
			in("al") data,
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
/// Waits for an I/O operation to complete.
pub unsafe fn io_wait() {
	unsafe { outb(0x80, 0) };
}
