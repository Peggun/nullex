// memory.rs

/*
Memory module for the kernel.
*/

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
	PhysAddr,
	VirtAddr,
	structures::paging::{
		FrameAllocator,
		Mapper,
		OffsetPageTable,
		Page,
		PageTable,
		PageTableFlags,
		PhysFrame,
		Size4KiB
	}
};

use crate::println;

pub fn map_apic(
	mapper: &mut impl Mapper<Size4KiB>,
	frame_allocator: &mut impl FrameAllocator<Size4KiB>
) {
	println!("[Info] Mapping APIC Timer...");

	let apic_phys_start = 0xFEE00000;
	let apic_page = Page::containing_address(VirtAddr::new(apic_phys_start));
	let apic_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

	unsafe {
		mapper
			.map_to(
				apic_page,
				PhysFrame::containing_address(PhysAddr::new(apic_phys_start)),
				apic_flags,
				frame_allocator
			)
			.unwrap()
			.flush();
	}

	println!("[Info] Done.");
}

/// A FrameAllocator that returns usable frames from the bootloader's memory
/// map.
pub struct BootInfoFrameAllocator {
	memory_map: &'static MemoryMap,
	next: usize
}

impl BootInfoFrameAllocator {
	/// Create a FrameAllocator from the passed memory map.
	pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
		BootInfoFrameAllocator {
			memory_map,
			next: 0
		}
	}

	/// Returns an iterator over the usable frames specified in the memory map.
	fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
		// get usable regions from memory map
		let regions = self.memory_map.iter();
		let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
		// map each region to its address range
		let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
		// transform to an iterator of frame start addresses
		let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
		// create `PhysFrame` types from the start addresses
		frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
	}
}

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
	fn allocate_frame(&mut self) -> Option<PhysFrame> {
		let frame = self.usable_frames().nth(self.next);
		self.next += 1;
		frame
	}
}

/// Translates the given virtual address to the mapped physical address, or
/// `None` if the address is not mapped.
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
	translate_addr_inner(addr, physical_memory_offset)
}

/// function that is called by `translate_addr`.
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
	use x86_64::{registers::control::Cr3, structures::paging::page_table::FrameError};

	// read the active level 4 frame from the CR3 register
	let (level_4_table_frame, _) = Cr3::read();

	let table_indexes = [
		addr.p4_index(),
		addr.p3_index(),
		addr.p2_index(),
		addr.p1_index()
	];
	let mut frame = level_4_table_frame;

	// traverse the multi-level page table
	for &index in &table_indexes {
		// convert the frame into a page table reference
		let virt = physical_memory_offset + frame.start_address().as_u64();
		let table_ptr: *const PageTable = virt.as_ptr();
		let table = unsafe { &*table_ptr };

		// read the page table entry and update `frame`
		let entry = &table[index];
		frame = match entry.frame() {
			Ok(frame) => frame,
			Err(FrameError::FrameNotPresent) => return None,
			Err(FrameError::HugeFrame) => panic!("huge pages not supported")
		};
	}

	// calculate the physical address by adding the page offset
	Some(frame.start_address() + u64::from(addr.page_offset()))
}

/// Returns a mutable reference to the active level 4 table.
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
	use x86_64::registers::control::Cr3;

	let (level_4_table_frame, _) = Cr3::read();

	let phys = level_4_table_frame.start_address();
	let virt = physical_memory_offset + phys.as_u64();
	let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

	unsafe { &mut *page_table_ptr } // unsafe
}

/// Creates an example mapping for the given page to frame `0xb8000`.
pub fn create_example_mapping(
	page: Page,
	mapper: &mut OffsetPageTable,
	frame_allocator: &mut impl FrameAllocator<Size4KiB>
) {
	use x86_64::structures::paging::PageTableFlags as Flags;

	let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
	let flags = Flags::PRESENT | Flags::WRITABLE;

	let map_to_result = unsafe {
		// FIXME: this is not safe, we do it only for testing
		mapper.map_to(page, frame, flags, frame_allocator)
	};
	map_to_result.expect("map_to failed").flush();
}

pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
	let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };
	unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}
