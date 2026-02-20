//!
//!  memory.rs
//!
//! Memory module for the kernel.
//!

use alloc::{boxed::Box, vec::Vec};

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
		Size4KiB,
		Translate
	}
};

use crate::{
	PHYS_MEM_OFFSET,
	allocator::{self, ALLOCATOR_INFO},
	arch::x86_64::bootinfo::{MemoryMap, MemoryRegionType},
	lazy_static,
	println,
	serial_println,
	utils::{
		multiboot2::{__link_phys_base, _end, compute_phys_map_offset},
		mutex::SpinMutex
	}
};

lazy_static! {
	/// Static reference to the Physical Memory Map offset.
	pub static ref PAGE_OFFSET: SpinMutex<u64> =
		SpinMutex::new(unsafe { compute_phys_map_offset() });
}

static mut NEXT_DMA_VIRT: u64 = 0x5555_0000_0000;

#[derive(Clone, Copy)]
/// Structure representing a buffer of DMA (Direct Memory Access) information
pub struct DmaBuffer {
	/// The Physical Address of the DMA buffer
	pub phys: PhysAddr,
	/// The Virtual Address of the DMA buffer
	pub virt: VirtAddr,
	/// The length of the DMA buffer
	pub len: usize
}

/// Initializes the global allocator with the specified strategy in `allocator.rs`
// todo! eventually kernel config for types of allocators
pub fn init_global_alloc(
	mut mapper: OffsetPageTable<'static>,        
	mut frame_allocator: BootInfoFrameAllocator  
) -> Result<(), &'static str> {
	match allocator::init_heap(&mut mapper, &mut frame_allocator) {
		Ok(()) => println!("[Info] Heap pages mapped successfully"),
		Err(e) => panic!("Heap mapping failed: {:?}", e)
	}

	unsafe {
		allocator::LOCAL_HEAP_ALLOCATOR
			.lock()
			.init(allocator::HEAP_START, allocator::HEAP_SIZE);

		let allocator_ref = &allocator::LOCAL_HEAP_ALLOCATOR;
		ALLOCATOR_INFO.strategy.write().replace(allocator_ref);
	}

	println!("[Info] Heap Initialized. Promoting structures to 'static...");

	let static_frame_alloc = Box::leak(Box::new(frame_allocator));
	let static_mapper = Box::leak(Box::new(mapper));

	*ALLOCATOR_INFO.frame_allocator.lock() = Some(static_frame_alloc);
	*ALLOCATOR_INFO.mapper.lock() = Some(static_mapper);

	Ok(())
}

/// Maps the APIC Timer to valid addresses for use
pub fn map_apic(
	mapper: &mut impl Mapper<Size4KiB>,
	frame_allocator: &mut impl FrameAllocator<Size4KiB>,
	physical_memory_offset: VirtAddr
) {
	println!("[Info] Mapping APIC Timer...");

	const APIC_PHYS_START: u64 = 0xFEE0_0000u64;
	let apic_phys = PhysAddr::new(APIC_PHYS_START);
	let apic_frame = PhysFrame::containing_address(apic_phys);

	// compute the virtual address we actually use to access physical memory
	let apic_virt = VirtAddr::new(physical_memory_offset.as_u64() + APIC_PHYS_START);
	let apic_page = Page::containing_address(apic_virt);

	let apic_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

	unsafe {
		mapper
			.map_to(apic_page, apic_frame, apic_flags, frame_allocator)
			.unwrap()
			.flush();
	}

	println!("[Info] Done.");
}

/// Maps the IOAPIC timer to valid addresses for use
pub fn map_ioapic(
	mapper: &mut impl Mapper<Size4KiB>,
	frame_allocator: &mut impl FrameAllocator<Size4KiB>,
	physical_memory_offset: VirtAddr
) {
	println!("[Info] Mapping IOAPIC...");

	const IOAPIC_PHYS_START: u64 = 0xFEC0_0000u64;
	let ioapic_phys = PhysAddr::new(IOAPIC_PHYS_START);
	let ioapic_frame = PhysFrame::containing_address(ioapic_phys);

	// virtual address that maps to the physical IOAPIC
	let ioapic_virt = VirtAddr::new(physical_memory_offset.as_u64() + IOAPIC_PHYS_START);
	let ioapic_page = Page::containing_address(ioapic_virt);

	let ioapic_flags =
		PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

	unsafe {
		mapper
			.map_to(ioapic_page, ioapic_frame, ioapic_flags, frame_allocator)
			.unwrap()
			.flush();
	}

	println!("[Info] IOAPIC mapped at virt {:#X}", ioapic_virt.as_u64());
}

/// A FrameAllocator that returns usable frames from the bootloader's memory
/// map.
pub struct BootInfoFrameAllocator {
	memory_map: &'static MemoryMap,
	next: usize
}

impl BootInfoFrameAllocator {
	/// Create a FrameAllocator from the passed memory map.
	pub fn init(memory_map: &'static MemoryMap) -> Self {
		BootInfoFrameAllocator {
			memory_map,
			next: 0
		}
	}

	/// Returns an iterator over the usable frames specified in the memory map.
	fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
		let regions = self.memory_map.iter();
		let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);

		// kernel bounds (physical addresses)
		let kernel_start = unsafe { &__link_phys_base as *const _ as u64 };
		let kernel_end = unsafe { &_end as *const _ as u64 };

		let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());

		let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

		frame_addresses
			.filter(move |addr| (addr < &kernel_start) || (addr >= &kernel_end))
			.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
	}
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
	fn allocate_frame(&mut self) -> Option<PhysFrame> {
		let frame = self.usable_frames().nth(self.next);
		self.next += 1;
		frame
	}
}

/// Translates the given virtual address to the mapped physical address, or
/// `None` if the address is not mapped.
/// # Safety
/// We need all memory mapped at `physical_memory_offset`.
pub unsafe fn virt_to_phys(addr: VirtAddr) -> Option<PhysAddr> {
	let pmo = *PHYS_MEM_OFFSET.lock();
	let level_4_table = unsafe { active_level_4_table(pmo) };
	unsafe { OffsetPageTable::new(level_4_table, pmo) }.translate_addr(addr)
}

/// Returns a mutable reference to the active level 4 table.
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
	use x86_64::registers::control::Cr3;

	let (level_4_table_frame, _) = Cr3::read();

	let phys = level_4_table_frame.start_address();
	let virt = physical_memory_offset + phys.as_u64();
	let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

	unsafe { &mut *page_table_ptr }
}

/// Translates a physical address to a virtual one
/// 
/// # Safety
/// Physical address needs to be mapped, if not, the virtual address returned will be invalid.
pub unsafe fn phys_to_virt(addr: PhysAddr) -> VirtAddr {
	VirtAddr::new(addr.as_u64().wrapping_add(*PAGE_OFFSET.lock()))
}

/// # Safety
/// We need some memory mapped at `physical_memory_offset`.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
	let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };
	unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}

/// Allocates a direct memory access block of `size` bytes.
pub fn dma_alloc(size: usize) -> Option<(VirtAddr, PhysAddr)> {
	let mut mapper_binding = ALLOCATOR_INFO.mapper.lock();
	let mapper_slot = mapper_binding.as_mut().unwrap();
	let mut frame_binding = ALLOCATOR_INFO.frame_allocator.lock();
	let frame_slot = frame_binding.as_mut().unwrap();

	let page_count = (size + 4095) / 4096;

	let mut frames = Vec::new();
	for _ in 0..page_count {
		if let Some(frame) = frame_slot.allocate_frame() {
			frames.push(frame);
		} else {
			return None;
		}
	}

	for i in 1..frames.len() {
		if frames[i].start_address().as_u64() != frames[i - 1].start_address().as_u64() + 4096 {
			serial_println!(
				"[DMA] Allocation failed: Frames not contiguous at index {}",
				i
			);
			return None;
		}
	}

	let first_phys = frames[0].start_address();

	let virt_addr = VirtAddr::new(unsafe { NEXT_DMA_VIRT });
	unsafe {
		NEXT_DMA_VIRT += (page_count as u64) * 4096;
	}

	for (i, frame) in frames.iter().enumerate() {
		let va = virt_addr + (i as u64) * 4096;
		let page = Page::containing_address(va);

		let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

		unsafe {
			mapper_slot
				.map_to(page, *frame, flags, *frame_slot)
				.expect("Failed to map DMA page")
				.flush();
		}
	}

	Some((virt_addr, first_phys))
}
