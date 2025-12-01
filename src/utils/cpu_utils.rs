use core::{arch::asm, sync::atomic::AtomicU32};

// from https://board.flatassembler.net/topic.php?p=240655
// but translated to x64 with some AI magic, because fuck asm
/// # Safety
/// idfk uhh its raw asm maybe it does some cpu state fuckery
pub unsafe fn get_cpu_clock() -> u32 {
	let result: u32;
	unsafe {
		asm!(
			// save flags and disable interrupts
			"pushfq",
			"cli",

			// set RTC register B bit 2
			"mov al, 0x0b",
			"out 0x70, al",
			"in al, 0x71",
			"mov bl, al",
			"mov al, 0x0b",
			"out 0x70, al",
			"mov al, bl",
			"or al, 0x04",
			"out 0x71, al",

			// wait until RTC register A < 0x38 then capture current value into bl
			"2:",
			"mov al, 0",
			"out 0x70, al",
			"in al, 0x71",
			"cmp al, 0x38",
			"jge 2b",
			"mov bl, al",

			// wait for next increment of RTC value (bl -> next value)
			"3:",
			"mov al, 0",
			"out 0x70, al",
			"in al, 0x71",
			"cmp al, bl",
			"jle 3b",
			"mov bl, al",

			// first timestamp
			"rdtsc",
			// save first timestamp into r10d:r11d (low -> r10d, high -> r11d)
			"mov r10d, eax",
			"mov r11d, edx",

			// wait until RTC changes again
			"4:",
			"mov al, 0",
			"out 0x70, al",
			"in al, 0x71",
			"cmp al, bl",
			"je 4b",

			// second timestamp
			"rdtsc",

			// re-enable interrupts (we will restore flags shortly)
			"sti",

			// compute 64-bit delta: (edx:eax) - (r11d:r10d)
			"sub eax, r10d",   // low-word subtract
			"sbb edx, r11d",   // high-word subtract with borrow -> edx:eax now = delta

			// return lower 32 bits of delta in `eax` (captured via lateout below)
			// restore RTC register B to previous value (clear bit 2)
			"mov al, 0x0b",
			"out 0x70, al",
			"in al, 0x71",
			"mov bl, al",
			"mov al, 0x0b",
			"out 0x70, al",
			"mov al, bl",
			"and al, 11111011b", // clear bit 2
			"out 0x71, al",

			// restore flags
			"popfq",

			// outputs / clobbers
			lateout("eax") result,      // return value (low 32 bits of delta)
			out("rdx") _,               // rdx/edx clobbered by rdtsc/ops
			out("bl") _,                // we used bl
			out("r10") _, out("r11") _, // temporaries used to store first timestamp
		);
	}

	result
}
