// allocator.rs

/*
Heap allocator module for the kernel.
*/

use core::alloc;

use linked_list::LinkedListAllocator;

pub mod bump;
pub mod fixed_size_block;
pub mod linked_list;

use x86_64::structures::paging::{
	FrameAllocator,
	Mapper,
	Page,
	PageTableFlags,
	Size4KiB,
	mapper::MapToError
};

use crate::{
	println,
	utils::mutex::{SpinMutex, SpinMutexGuard}
};

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 2 * 1024 * 1024;

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
	use x86_64::VirtAddr;

	println!("[Info] Initializing Heap...");

	// Basic sanity checks
	#[expect(clippy::assertions_on_constants)]
	{
		assert!(HEAP_SIZE > 0, "HEAP_SIZE must be > 0");
		assert!(
			HEAP_START.is_multiple_of(4096),
			"HEAP_START must be page-aligned"
		);
	}

	// Use u64 for address math to match VirtAddr
	let heap_start_u64 = HEAP_START as u64;
	// compute last byte in heap safely
	let heap_end_u64 = heap_start_u64
		.checked_add(HEAP_SIZE as u64)
		.and_then(|v| v.checked_sub(1))
		.expect("HEAP_START + HEAP_SIZE overflow");

	// Check canonicalness instead of letting VirtAddr::new panic invisibly
	if VirtAddr::try_new(heap_start_u64).is_err() {
		panic!(
			"HEAP_START (0x{:x}) is not a canonical virtual address",
			heap_start_u64
		);
	}
	if VirtAddr::try_new(heap_end_u64).is_err() {
		panic!(
			"HEAP_END (0x{:x}) is not a canonical virtual address",
			heap_end_u64
		);
	}

	let start_page = Page::<Size4KiB>::containing_address(VirtAddr::new(heap_start_u64));
	let end_page = Page::<Size4KiB>::containing_address(VirtAddr::new(heap_end_u64));

	// Extra sanity
	let start_index = start_page.start_address().as_u64() / 4096;
	let end_index = end_page.start_address().as_u64() / 4096;
	assert!(start_index <= end_index, "heap start page > heap end page");

	let num_pages = end_index - start_index + 1;
	// avoid absurd page counts (protect against bad arithmetic)
	let max_reasonable_pages: u64 = 10 * 1024 * 1024; // ~10M pages (≈40GB)
	assert!(
		num_pages <= max_reasonable_pages,
		"heap range too large: {} pages",
		num_pages
	);

	println!(
		"[Info] heap: start=0x{:x}, end=0x{:x}, pages={}, start_page=0x{:x}, end_page=0x{:x}",
		heap_start_u64,
		heap_end_u64,
		num_pages,
		start_page.start_address().as_u64(),
		end_page.start_address().as_u64()
	);

	// Map pages one-by-one (safe, explicit)
	for page_index in start_index..=end_index {
		let va = VirtAddr::new(page_index * 4096);
		let page = Page::containing_address(va);

		let frame = frame_allocator
			.allocate_frame()
			.ok_or(MapToError::FrameAllocationFailed)?;
		let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

		unsafe {
			mapper.map_to(page, frame, flags, frame_allocator)?.flush();
		}
	}

	unsafe {
		ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
	}

	println!("[Info] Heap initialized ({} pages).", num_pages);
	Ok(())
}

/// A wrapper around spin::Mutex to permit trait implementations.
pub struct Locked<A> {
	inner: SpinMutex<A>
}

impl<A> Locked<A> {
	pub const fn new(inner: A) -> Self {
		Locked {
			inner: SpinMutex::new(inner)
		}
	}

	pub fn lock(&'_ self) -> SpinMutexGuard<'_, A> {
		self.inner.lock()
	}
}

/// Align the given address `addr` upwards to alignment `align`.
///
/// Requires that `align` is a power of two.
fn align_up(addr: usize, align: usize) -> usize {
	(addr + align - 1) & !(align - 1)
}

#[cfg(feature = "test")]
pub mod tests {
	use crate::{utils::ktest::TestError, allocator::align_up};

	pub fn test_align_up_already_aligned() -> Result<(), TestError> {
		let a = 0x1000usize;
		let aligned = align_up(a, 0x1000);
		assert_eq!(aligned, 0x1000);
		Ok(())
	}
	crate::create_test!(test_align_up_already_aligned);

	pub fn test_align_up_non_aligned() -> Result<(), TestError> {
		let a = 0x1001usize;
		let aligned = align_up(a, 0x1000);
		assert_eq!(aligned, 0x2000);
		Ok(())
	}
	crate::create_test!(test_align_up_non_aligned);
}
