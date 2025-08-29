// allocator.rs

/*
Heap allocator module for the kernel.
*/

use core::alloc;

use linked_list::LinkedListAllocator;

pub mod bump;
pub mod fixed_size_block;
pub mod linked_list;

use x86_64::{
	VirtAddr,
	structures::paging::{
		FrameAllocator,
		Mapper,
		Page,
		PageTableFlags,
		Size4KiB,
		mapper::MapToError
	}
};

use crate::println;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 1024 * 1024;

// fixed is better performance wise.
#[global_allocator]
static ALLOCATOR: Locked<LinkedListAllocator> = Locked::new(LinkedListAllocator::new());

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::Layout) -> ! {
	panic!("Allocation error: {:?}", layout)
}

pub fn init_heap(
	mapper: &mut impl Mapper<Size4KiB>,
	frame_allocator: &mut impl FrameAllocator<Size4KiB>
) -> Result<(), MapToError<Size4KiB>> {
	println!("[Info] Initializing Heap...");
	let page_range = {
		let heap_start = VirtAddr::new(HEAP_START as u64);
		let heap_end = heap_start + HEAP_SIZE - 1u64;
		let heap_start_page = Page::containing_address(heap_start);
		let heap_end_page = Page::containing_address(heap_end);
		Page::range_inclusive(heap_start_page, heap_end_page)
	};

	for page in page_range {
		let frame = frame_allocator
			.allocate_frame()
			.ok_or(MapToError::FrameAllocationFailed)?;
		let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
		unsafe { mapper.map_to(page, frame, flags, frame_allocator)?.flush() };
	}

	unsafe {
		ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
	}

	println!("[Info] Done.");
	Ok(())
}

/// A wrapper around spin::Mutex to permit trait implementations.
pub struct Locked<A> {
	inner: spin::Mutex<A>
}

impl<A> Locked<A> {
	pub const fn new(inner: A) -> Self {
		Locked {
			inner: spin::Mutex::new(inner)
		}
	}

	pub fn lock(&'_ self) -> spin::MutexGuard<'_, A> {
		self.inner.lock()
	}
}

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
fn align_up(addr: usize, align: usize) -> usize {
	(addr + align - 1) & !(align - 1)
}
