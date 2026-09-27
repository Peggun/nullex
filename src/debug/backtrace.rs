use core::mem::size_of;

use crate::{
	allocator::ALLOCATOR_INFO,
	debug::ndm::{NDM_DATA, NDMEntry},
	serial_println
};

// https://wiki.osdev.org/Stack_Trace
#[repr(C)]
pub struct StackTrace {
	pub rbp: *const StackTrace,
	pub rip: u64
}

pub fn trace_stack_trace(n_frames: usize) {
	let entry_size = size_of::<NDMEntry>();

	if entry_size != 144 {
		serial_println!(
			"[ERROR] Invalid NDM entry size: expected 144 bytes, got {} bytes",
			entry_size,
		);
		return;
	}

	let dataset: &[NDMEntry] = bytemuck::cast_slice::<u8, NDMEntry>(&NDM_DATA);

	let mut stack_frame: *const StackTrace;

	unsafe {
		core::arch::asm!(
			"mov {}, rbp",
			out(reg) stack_frame,
		);

		serial_println!();
		serial_println!("========== NULLEX STACK TRACE ==========");
		serial_println!("Requested frames : {}", n_frames);
		serial_println!("NDM entries      : {}", dataset.len());
		serial_println!("Initial RBP      : {:#018x}", stack_frame as usize);
		serial_println!("========================================");

		for frame_index in 0..n_frames {
			if stack_frame.is_null() {
				serial_println!("  <stack terminated: null frame pointer>");
				break;
			}

			let rip = (*stack_frame).rip;
			if rip < 0xffff800000000000 {
				serial_println!("  <stack terminated: invalid kernel address {:#x}>", rip);
				break;
			}

			let lookup_addr = if frame_index == 0 {
				rip
			} else {
				rip.saturating_sub(1)
			};

			serial_println!();
			serial_println!("#{:<3} {:#018x}", frame_index, rip,);

			match get_info_from_address(lookup_addr, dataset) {
				Some(info) => {
					let file_name = fixed_str(&info.file_name);
					let func_name = fixed_str(&info.func_name);

					serial_println!("      address  : {:#018x}", info.address);
					serial_println!("      function : {}", func_name);
					serial_println!("      location : {}:{}", file_name, info.line);
				}

				None => {
					serial_println!("      <unknown symbol>");
				}
			}

			stack_frame = (*stack_frame).rbp;
		}

		serial_println!();
		serial_println!("========== END STACK TRACE =============");
		serial_println!();
	}
}

fn fixed_str(buffer: &[u8]) -> &str {
	let len = buffer
		.iter()
		.position(|&byte| byte == 0)
		.unwrap_or(buffer.len());

	core::str::from_utf8(&buffer[..len]).unwrap_or("<invalid utf-8>")
}

fn get_info_from_address(address: u64, dataset: &[NDMEntry]) -> Option<NDMEntry> {
	match dataset.binary_search_by_key(&address, |entry| entry.address) {
		Ok(index) => Some(dataset[index]),

		Err(0) => None,

		Err(index) => Some(dataset[index - 1])
	}
}
