// https://en.eeworld.com.cn/mp/rrgeek/a101932.jspx#:~:text=Some%20CPU%20architectures%20(typically%20the,/O%2Dmapped%20mode.%22

use alloc::vec::Vec;

use crate::{lazy_static, utils::mutex::SpinMutex};

lazy_static! {
	pub static ref IO_ALLOC: SpinMutex<IoAllocator> = SpinMutex::new(IoAllocator::new(0x0000, 0x10000)); // 64KiB
}

#[derive(Debug, Clone, Copy)]
pub struct IoRange {
	pub start: u32,
	pub size: u32
}

impl IoRange {
	pub fn end(&self) -> u32 {
		self.start.wrapping_add(self.size)
	}
}

pub struct IoAllocator {
	pub free: Vec<IoRange>
}

impl IoAllocator {
	/// Creates a new `IoAllocator` and initialises the whole region
	/// (0x0000 - 0xFFFF) with some reserve calls to prevent overlapping
	/// of current ports that are in use, but are currently not implemented
	/// in the source code file. Like the PIT ports for example, there is no
	/// specific `IoRange` in the PIT file, so we just (at the moment) set a
	/// reserve call here.
	pub fn new(start: u32, size: u32) -> Self {
		let mut v = Vec::new();
		v.push(IoRange {
			start,
			size
		});

		let mut a = Self {
			free: v
		};

		// https://wiki.osdev.org/I/O_Ports
		a.reserve(0x0000, 0x0020); // DMA controller
		a.reserve(0x0020, 0x0002); // PIC1
		a.reserve(0x0040, 0x0008); // PIT
		a.reserve(0x0060, 0x0005); // 8042
		a.reserve(0x0070, 0x0002); // CMOS/RTC
		a.reserve(0x0080, 0x0010); // DMA page regs
		a.reserve(0x0092, 0x0001); // fast A20
		a.reserve(0x00A0, 0x0002); // PIC2
		a.reserve(0x00C0, 0x0020); // DMA2 / sound region
		a.reserve(0x00E9, 0x0001); // port E9
		a.reserve(0x0170, 0x0008); // secondary ATA
		a.reserve(0x01F0, 0x0008); // primary ATA
		a.reserve(0x0278, 0x0003); // parallel
		a.reserve(0x02F8, 0x0008); // COM2
		a.reserve(0x03B0, 0x0030); // VGA legacy
		a.reserve(0x03F0, 0x0008); // floppy
		a.reserve(0x03F8, 0x0008); // COM1
		a.reserve(0x0CF8, 0x0008); // PCI config (0xCF8/0xCFC)

		a
	}

	/// Allocates `size` bytes with `align` alignment. `align` **MUST** be a
	/// power of 2.
	pub fn alloc(&mut self, size: u32, align: u32) -> Option<u32> {
		if size == 0 || align == 0 {
			return None;
		}
		if !align.is_power_of_two() {
			return None;
		}

		let mut i = 0;
		while i < self.free.len() {
			let r = self.free[i];
			let align_mask = align - 1;

			// align_up = (r.start + align - 1) & !(align-1)
			// we use wrapping_* functions to see if aligned_start has become < r.start,
			// thus indicating a overflow of the vector, (0xFFFF)
			let aligned_start = r.start.wrapping_add(align.wrapping_sub(1)) & !align_mask;

			if aligned_start < r.start {
				// overflowed
				i += 1;
				continue;
			}

			// check whether the aligned block fits
			let required_end = aligned_start.wrapping_add(size);
			if required_end <= r.end() {
				// we can allocate at aligned_start

				// allocation is at the very start of `r`
				if aligned_start == r.start {
					if size == r.size {
						// exact fit
						self.free.remove(i);
					} else {
						// move start forward
						self.free[i].start = required_end;
						self.free[i].size = self.free[i].size.wrapping_sub(size);
					}
				} else {
					// allocation is inside or at the end of `r`. `r` being 0x0000 - 0xFFFF
					// split into two ranges, left and right
					// here is a diagram because I was confused at first.
					/*

					Before allocation:
					Free range [r]:  |------- r.start -------- r.end() -------|
									 [            Available space             ]

					After allocation:
									|-- left --|[==== allocated ====]|-- right --|
									^          ^                     ^           ^
								r.start   aligned_start         required_end    r.end()
					*/
					let left_size = aligned_start.wrapping_sub(r.start);
					let right_end = r.end();
					let right_size = right_end.wrapping_sub(required_end);

					self.free[i].size = left_size;

					if right_size > 0 {
						let new_range = IoRange {
							start: required_end,
							size: right_size
						};
						self.free.insert(i + 1, new_range);
					}
				}

				return Some(aligned_start);
			}

			i += 1;
		}

		// no suitable free range found
		None
	}

	/// Free an already allocated range between base and size. Also merges two
	/// consecutive free ranges.
	pub fn free(&mut self, base: u32, size: u32) {
		if size == 0 {
			return;
		}
		let end = base.wrapping_add(size);

		let mut pos = 0usize;
		while pos < self.free.len() && self.free[pos].start < base {
			pos += 1;
		}

		// try to merge with previous
		if pos > 0 {
			let prev = self.free[pos - 1];
			if prev.end() == base {
				self.free[pos - 1].size = prev.size.wrapping_add(size);

				// maybe also merge with next
				if pos < self.free.len() && self.free[pos].start == end {
					let next_size = self.free[pos].size;
					self.free[pos - 1].size = self.free[pos - 1].size.wrapping_add(next_size);

					// remove because if merge with previous (left) and next (right) we are the
					// middle position, thus no point keeping it so we remove it.
					self.free.remove(pos);
				}

				return;
			}
		}

		// try to merge with next
		if pos < self.free.len() && self.free[pos].start == end {
			self.free[pos].start = base;
			self.free[pos].size = self.free[pos].size.wrapping_add(size);
			return;
		}

		// just insert a new free range at pos
		self.free.insert(pos, IoRange {
			start: base,
			size
		});
	}

	/// Reserve the given [base, base+size] range from the free list.
	pub fn reserve(&mut self, base: u32, size: u32) {
		if size == 0 {
			return;
		}
		let mut i = 0usize;
		let end = base.wrapping_add(size);

		while i < self.free.len() {
			let r = self.free[i];

			let r_start = r.start;
			let r_end = r.end();

			// no overlap
			if r_end <= base || r_start >= end {
				i += 1;
				continue;
			}

			// full cover
			if base <= r_start && end >= r_end {
				self.free.remove(i);
				continue;
			}

			// reservation trims the left side of free block
			if base <= r_start && end > r_start && end < r_end {
				let new_start = end;
				let new_size = r_end.wrapping_sub(end);

				self.free[i].start = new_start;
				self.free[i].size = new_size;
				i += 1;
				continue;
			}

			// reservation trims the right side of free block
			if base > r_start && base < r_end && end >= r_end {
				let new_size = base.wrapping_sub(r_start);
				self.free[i].size = new_size;
				i += 1;
				continue;
			}

			// reservation is inside this free block
			if base > r_start && end < r_end {
				let left_size = base.wrapping_sub(r_start);
				let right_start = end;
				let right_size = r_end.wrapping_sub(end);

				self.free[i].size = left_size;

				let right = IoRange {
					start: right_start,
					size: right_size
				};
				self.free.insert(i + 1, right);
			}

			i += 1;
		}
	}
}
