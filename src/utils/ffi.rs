use core::{
	alloc::Layout,
	ptr::{null_mut, write_unaligned}
};

use crate::allocator::ALLOCATOR_INFO;

// dont change this yet, this makes the alignment math incorrect.
// todo: fix that.
pub const MALLOC_ALIGN: usize = 16;

#[repr(C)]
pub struct MallocHeader {
	pub size: usize
}

pub const HEADER_SIZE: usize = size_of::<MallocHeader>();
pub const HEADER_PADDED: usize = (HEADER_SIZE + MALLOC_ALIGN - 1) & !(MALLOC_ALIGN - 1);

/// Kernel side memory allocation, made mainly for "C" ffi functions, but can
/// be used within rust main code.
pub fn malloc(size: u32) -> *mut u8 {
	let size = size as usize;

	// total size which the memory would be
	// including the padded header. like this
	// if the user wanted 100 bytes.
	//     +------------------+----------------------+------------------------+
	//     | MallocHeader     | User's 100 bytes     |         Empty          |
	//     +------------------+----------------------+------------------------+
	//     ^                  ^                                               ^
	//     base               returned pointer                max size of usize
	let total = match HEADER_PADDED.checked_add(size) {
		Some(v) => v,
		None => return null_mut()
	};

	// layout favouring the alignment of the highest value between 16, and the
	// alignment of `MallocHeader`
	let layout = match Layout::from_size_align(total, MALLOC_ALIGN.max(align_of::<MallocHeader>()))
	{
		Ok(layout) => layout,
		Err(_) => return null_mut()
	};

	let strategy = ALLOCATOR_INFO.strategy.read();
	if let Some(strategy) = &*strategy {
		unsafe {
			// allocates the full memory, however the address returns the first address of
			// the allocated memory block, which is the start of the `MallocHeader`.
			let base = strategy.alloc(layout);
			if base.is_null() {
				return null_mut();
			}

			let header = base as *mut MallocHeader;
			// write the `MallocHeader` directly to the header memory address
			// we don't need write_unaligned because we check if the header is
			// aligned previously, and this slightly improves runtime.
			core::ptr::write(header, MallocHeader {
				size
			});

			// we want to return the starting address of the user's bytes, not the header
			// thus we return the base address add the size of the padded header, which is
			// the address of the data of the memory. refer to the diagram above.
			return base.add(HEADER_PADDED) as *mut u8;
		}
	}

	null_mut()
}

/// Kernel side memory de-allocation, made mainly for "C" ffi functions, but can
/// be used within rust main code.
pub fn free(ptr: *mut u8) {
	if ptr.is_null() {
		return;
	}

	let strategy = ALLOCATOR_INFO.strategy.read();
	if let Some(strategy) = &*strategy {
		unsafe {
			// ptr is the pointer to the start of memory AFTER the MallocHeader.
			let user_ptr = ptr as *mut u8;

			// therefore to get the entire memory region, we need to move backwards
			// the size of the padded header to retrieve the entire memory block.
			let base = user_ptr.sub(HEADER_PADDED);

			let header = base as *const MallocHeader;
			// same thing here, we know that the header is aligned, so we can read safely.
			let size = core::ptr::read(header).size;

			// calculate the total size of the memory including the header region.
			let total = match HEADER_PADDED.checked_add(size) {
				Some(v) => v,
				None => return
			};

			let layout = match Layout::from_size_align(
				total,
				MALLOC_ALIGN.max(align_of::<MallocHeader>())
			) {
				Ok(layout) => layout,
				Err(_) => return
			};

			strategy.dealloc(base, layout);
		}
	}

	return;
}
