// this code mainly comes from here
// https://github.com/foliagecanine/tritium-os/blob/d8b78298f828c0745a480d309aceb4fd503c421f/kernel/arch/i386/sys/pit.c#L9
// which i find here
// https://forum.osdev.org/viewtopic.php?t=37296

use core::arch::asm;

use crate::common::ports::port_byte_out;

static mut FREQUENCY: u32 = 0;
static mut TICKS: u64 = 0;

pub fn pit_tick() {
	unsafe { TICKS += 1 };
}

pub fn init_pit(freq: u32) {
	unsafe { FREQUENCY = freq };

	let pit_freq = 1193181 / freq; // whats this number?
	port_byte_out(0x43, 0x34);
	port_byte_out(0x40, pit_freq as u8);
	port_byte_out(0x40, (pit_freq >> 8) as u8);
}

pub fn pit_sleep(ms: u32) {
	unsafe {
		let end_ticks = TICKS + ((ms * FREQUENCY) as u64 / 1000);
		while TICKS < end_ticks {
			asm!("nop");
		}
	}
}
