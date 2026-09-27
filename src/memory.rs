//!
//!  memory.rs
//!
//! Memory module for the kernel.

// src/memory/
pub mod user;
pub mod volatile;

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
		Translate,
		page::PageRange
	}
};

use crate::{
	allocator::{self, ALLOCATOR_INFO},
	arch::x86_64::bootinfo::{MemoryMap, MemoryRegionType},
	boot::multiboot2::{__kernel_phys_start, _end},
	error::NullexError,
	kassert,
	lazy_static,
	println,
	serial_println,
	sync::mutex::SpinMutex,
	task::address_space::AddressSpace
};

lazy_static! {
	/// Static reference to the Physical Memory Map offset.
	pub static ref PAGE_OFFSET: SpinMutex<u64> =
		SpinMutex::new(PHYS_MEM_OFFSET);
}

static mut NEXT_DMA_VIRT: u64 = 0xFFFF_E000_0000_0000;
const MAX_RESERVED_FRAMES: usize = 512;
pub const PHYS_MEM_OFFSET: u64 = 0xFFFF_FFFF_8000_0000;
pub const MMIO_BASE: u64 = 0xFFFF_FF80_0000_0000;

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

/// Initializes the global allocator with the specified strategy in
/// `allocator.rs`
// todo! eventually kernel config for types of allocators
pub fn init_global_alloc(
	mut mapper: OffsetPageTable<'static>,
	mut frame_allocator: BootInfoFrameAllocator
) -> Result<(), NullexError> {
	kassert!(
		allocator::init_heap(&mut mapper, &mut frame_allocator).is_ok(),
		"heap not allocated."
	);

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
	frame_allocator: &mut impl FrameAllocator<Size4KiB>
) {
	println!("[Info] Mapping APIC Timer...");

	const APIC_PHYS_START: u64 = 0xFEE0_0000u64;
	let apic_phys = PhysAddr::new(APIC_PHYS_START);
	let apic_frame = PhysFrame::containing_address(apic_phys);

	let apic_virt = VirtAddr::new(MMIO_BASE);
	let apic_page = Page::containing_address(apic_virt);

	let apic_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

	unsafe {
		mapper
			.map_to(apic_page, apic_frame, apic_flags, frame_allocator)
			.unwrap()
			.flush();
	}

	println!("[Info] APIC mapped at virt {:#X}", apic_virt.as_u64());
}

/// Maps the IOAPIC timer to valid addresses for use
pub fn map_ioapic(
	mapper: &mut impl Mapper<Size4KiB>,
	frame_allocator: &mut impl FrameAllocator<Size4KiB>
) {
	println!("[Info] Mapping IOAPIC...");

	const IOAPIC_PHYS_START: u64 = 0xFEC0_0000u64;
	let ioapic_phys = PhysAddr::new(IOAPIC_PHYS_START);
	let ioapic_frame = PhysFrame::containing_address(ioapic_phys);

	// virtual address that maps to the physical IOAPIC
	let ioapic_virt = VirtAddr::new(MMIO_BASE + 0x1000);
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
#[derive(Clone)]
pub struct BootInfoFrameAllocator {
	memory_map: &'static MemoryMap,
	next: u64,
	free_list: Vec<u64>,
	reserved_frames: [u64; MAX_RESERVED_FRAMES],
	reserved_count: usize
}

impl BootInfoFrameAllocator {
	/// Create a FrameAllocator from the passed memory map.
	pub fn init(memory_map: &'static MemoryMap) -> Self {
		let mut reserved_frames = [0u64; MAX_RESERVED_FRAMES];
		let mut reserved_count = 0usize;

		unsafe {
			let (pml4_frame, _) = x86_64::registers::control::Cr3::read();
			let pml4_phys = pml4_frame.start_address().as_u64();
			Self::collect_page_table_frames(pml4_phys, &mut reserved_frames, &mut reserved_count);
		}

		BootInfoFrameAllocator {
			memory_map,
			next: 0,
			free_list: Vec::new(),
			reserved_frames,
			reserved_count
		}
	}

	unsafe fn collect_page_table_frames(
		pml4_phys: u64,
		frames: &mut [u64; MAX_RESERVED_FRAMES],
		count: &mut usize
	) {
		use x86_64::structures::paging::PageTableFlags;

		let pml4_virt = unsafe { phys_to_virt(PhysAddr::new(pml4_phys)) };
		let pml4 = unsafe { &*(pml4_virt.as_ptr::<x86_64::structures::paging::PageTable>()) };

		if *count < MAX_RESERVED_FRAMES {
			frames[*count] = pml4_phys & !0xFFF;
			*count += 1;
		}

		for entry in pml4.iter() {
			if !entry.flags().contains(PageTableFlags::PRESENT) {
				continue;
			}
			if entry.flags().contains(PageTableFlags::HUGE_PAGE) {
				continue;
			}
			let Ok(pml3_frame) = entry.frame() else {
				continue;
			};
			let pml3_phys = pml3_frame.start_address().as_u64();

			if *count < MAX_RESERVED_FRAMES {
				frames[*count] = pml3_phys & !0xFFF;
				*count += 1;
			}

			let pml3_virt = unsafe { phys_to_virt(PhysAddr::new(pml3_phys)) };
			let pml3 = unsafe { &*(pml3_virt.as_ptr::<x86_64::structures::paging::PageTable>()) };
			for entry in pml3.iter() {
				if !entry.flags().contains(PageTableFlags::PRESENT) {
					continue;
				}
				if entry.flags().contains(PageTableFlags::HUGE_PAGE) {
					continue;
				}
				let Ok(pml2_frame) = entry.frame() else {
					continue;
				};
				let pml2_phys = pml2_frame.start_address().as_u64();

				if *count < MAX_RESERVED_FRAMES {
					frames[*count] = pml2_phys & !0xFFF;
					*count += 1;
				}

				let pml2_virt = unsafe { phys_to_virt(PhysAddr::new(pml2_phys)) };
				let pml2 =
					unsafe { &*(pml2_virt.as_ptr::<x86_64::structures::paging::PageTable>()) };
				for entry in pml2.iter() {
					if !entry.flags().contains(PageTableFlags::PRESENT) {
						continue;
					}
					if entry.flags().contains(PageTableFlags::HUGE_PAGE) {
						continue;
					}
					let Ok(pml1_frame) = entry.frame() else {
						continue;
					};

					if *count < MAX_RESERVED_FRAMES {
						frames[*count] = pml1_frame.start_address().as_u64() & !0xFFF;
						*count += 1;
					}
				}
			}
		}
	}

	fn is_reserved(&self, addr: u64) -> bool {
		let kernel_start = core::ptr::addr_of!(__kernel_phys_start) as u64;
		let kernel_end_virt = core::ptr::addr_of!(_end) as u64;
		const KERNEL_VIRT_BASE: u64 = 0xffff_ffff_8000_0000;
		let kernel_end = if kernel_end_virt >= KERNEL_VIRT_BASE {
			kernel_end_virt - KERNEL_VIRT_BASE
		} else {
			kernel_end_virt
		};

		if addr < kernel_start {
			return true;
		}

		if addr < kernel_end {
			return true;
		}

		for i in 0..self.reserved_count {
			if addr == self.reserved_frames[i] {
				return true;
			}
		}

		false
	}

	pub fn deallocate_frame(&mut self, frame: PhysFrame) {
		self.free_list.push(frame.start_address().as_u64());
	}
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
	fn allocate_frame(&mut self) -> Option<PhysFrame> {
		if let Some(addr) = self.free_list.pop() {
			let frame = PhysFrame::containing_address(PhysAddr::new(addr));
			unsafe {
				core::ptr::write_bytes(
					phys_to_virt(frame.start_address()).as_mut_ptr::<u8>(),
					0,
					4096
				);
			}
			return Some(frame);
		}
		for region in self.memory_map.iter() {
			if region.region_type != MemoryRegionType::Usable {
				continue;
			}

			let region_start = (region.range.start_addr() + 4095) & !4095;
			let region_end = region.range.end_addr() & !4095;
			let mut addr = self.next.max(region_start);

			while addr < region_end {
				if self.is_reserved(addr) {
					addr += 4096;
					self.next = addr;
					continue;
				}

				self.next = addr + 4096;
				let frame = PhysFrame::containing_address(PhysAddr::new(addr));

				unsafe {
					core::ptr::write_bytes(
						phys_to_virt(frame.start_address()).as_mut_ptr::<u8>(),
						0,
						4096
					);
				}

				return Some(frame);
			}
		}

		None
	}
}

/// Translates the given virtual address to the mapped physical address, or
/// `None` if the address is not mapped.
/// # Safety
/// We need all memory mapped at `physical_memory_offset`.
pub unsafe fn virt_to_phys(addr: VirtAddr) -> Option<PhysAddr> {
	let pmo = VirtAddr::new(PHYS_MEM_OFFSET);
	let level_4_table = unsafe { active_level_4_table(pmo) };
	unsafe { OffsetPageTable::new(level_4_table, pmo) }.translate_addr(addr)
}

/// Returns a mutable reference to the active level 4 table.
pub unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
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
/// Physical address needs to be mapped, if not, the virtual address returned
/// will be invalid.
pub unsafe fn phys_to_virt(addr: PhysAddr) -> VirtAddr {
	VirtAddr::new(addr.as_u64().wrapping_add(PHYS_MEM_OFFSET))
}

/// # Safety
/// We need some memory mapped at `physical_memory_offset`.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
	let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };
	unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}

/// Allocates a direct memory access block of `size` bytes.
pub fn dma_alloc(size: usize) -> Result<(VirtAddr, PhysAddr), NullexError> {
	let _kg = unsafe { crate::task::address_space::Cr3Guard::enter_kernel() };

	let mut mapper_binding = ALLOCATOR_INFO.mapper.lock();
	let mapper_slot = mapper_binding
		.as_mut()
		.ok_or(NullexError::MapperNotInitialized)?;
	let mut frame_binding = ALLOCATOR_INFO.frame_allocator.lock();
	let frame_slot = frame_binding
		.as_mut()
		.ok_or(NullexError::FrameAllocatorNotInitialized)?;

	let page_count = (size + 4095) / 4096;

	let mut frames = Vec::new();
	for _ in 0..page_count {
		if let Some(frame) = frame_slot.allocate_frame() {
			frames.push(frame);
		} else {
			return Err(NullexError::FrameAllocationFailed);
		}
	}

	for i in 1..frames.len() {
		if frames[i].start_address().as_u64() != frames[i - 1].start_address().as_u64() + 4096 {
			serial_println!(
				"[DMA] Allocation failed: Frames not contiguous at index {}",
				i
			);
			return Err(NullexError::DmaAllocFailed);
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
				.map_to(page, *frame, flags, *frame_slot)?
				.flush();
		}
	}

	Ok((virt_addr, first_phys))
}

/// Maps a range of memory within a `Process`'s `AddressSpace`.
pub fn map_range(
	addr_space: &mut AddressSpace,
	pages: PageRange,
	flags: PageTableFlags
) -> Result<(), NullexError> {
	let mut frame_binding = ALLOCATOR_INFO.frame_allocator.lock();
	let frame_allocator = frame_binding
		.as_mut()
		.ok_or(NullexError::FrameAllocatorNotInitialized)?;

	// we have to use a new mapper here because: global mapper: a view into whatever
	// page table CR3 is currently using not the the process's page table
	let table_ptr = unsafe { phys_to_virt(addr_space.page_table.start_address()) };
	let mut mapper = unsafe {
		OffsetPageTable::new(&mut *table_ptr.as_mut_ptr(), VirtAddr::new(PHYS_MEM_OFFSET))
	};

	for page in pages {
		let frame = frame_allocator
			.allocate_frame()
			.ok_or(NullexError::FrameAllocationFailed)?;

		unsafe {
			mapper.map_to(page, frame, flags, *frame_allocator)?.flush();
		}
	}

	Ok(())
}

#[cfg(feature = "test")]
pub mod tests {
	use x86_64::{
		PhysAddr,
		VirtAddr,
		structures::paging::{FrameAllocator, PhysFrame, Size4KiB}
	};

	use crate::{
		lazy_static,
		memory::{DmaBuffer, MAX_RESERVED_FRAMES, MemoryMap, PHYS_MEM_OFFSET},
		tassert,
		tassert_eq,
		tassert_ne,
		testing::ktest::TestError
	};

	lazy_static! {
		static ref DUMMY_MAP: MemoryMap = MemoryMap::new();
	}

	pub fn test_max_reserved_frames_value() -> Result<(), TestError> {
		tassert_eq!(MAX_RESERVED_FRAMES, 512);
		Ok(())
	}
	crate::create_test!(test_max_reserved_frames_value);

	pub fn test_dma_buffer_creation() -> Result<(), TestError> {
		let phys = PhysAddr::new(0x1000);
		let virt = VirtAddr::new(0xffff_8000_0000_1000);
		let len = 4096;
		let buffer = DmaBuffer {
			phys,
			virt,
			len
		};
		tassert_eq!(buffer.phys, phys);
		tassert_eq!(buffer.virt, virt);
		tassert_eq!(buffer.len, len);
		Ok(())
	}
	crate::create_test!(test_dma_buffer_creation);

	pub fn test_phys_to_virt_calculation() -> Result<(), TestError> {
		let phys = PhysAddr::new(0x10_0000);
		let virt = unsafe { crate::memory::phys_to_virt(phys) };
		tassert_eq!(virt.as_u64(), 0x10_0000 + PHYS_MEM_OFFSET);
		Ok(())
	}
	crate::create_test!(test_phys_to_virt_calculation);

	pub fn test_dma_alloc_page_count_calculation() -> Result<(), TestError> {
		let test_cases = [(1, 1), (4096, 1), (4097, 2), (8192, 2), (10000, 3), (0, 0)];
		for (size, expected_pages) in test_cases {
			let page_count = if size == 0 { 0 } else { (size + 4095) / 4096 };
			tassert_eq!(page_count, expected_pages, "Failed for size {}", size);
		}
		Ok(())
	}
	crate::create_test!(test_dma_alloc_page_count_calculation);

	pub fn test_dma_buffer_contiguity_check_logic() -> Result<(), TestError> {
		let frames_contiguous: [PhysFrame<Size4KiB>; 3] = [
			PhysFrame::containing_address(PhysAddr::new(0x1000)),
			PhysFrame::containing_address(PhysAddr::new(0x2000)),
			PhysFrame::containing_address(PhysAddr::new(0x3000))
		];
		for i in 1..frames_contiguous.len() {
			tassert_eq!(
				frames_contiguous[i].start_address().as_u64(),
				frames_contiguous[i - 1].start_address().as_u64() + 4096
			);
		}

		let frames_non_contiguous: [PhysFrame<Size4KiB>; 2] = [
			PhysFrame::containing_address(PhysAddr::new(0x1000)),
			PhysFrame::containing_address(PhysAddr::new(0x3000))
		];
		tassert_ne!(
			frames_non_contiguous[1].start_address().as_u64(),
			frames_non_contiguous[0].start_address().as_u64() + 4096
		);
		Ok(())
	}
	crate::create_test!(test_dma_buffer_contiguity_check_logic);

	pub fn test_next_dma_virt_is_valid() -> Result<(), TestError> {
		unsafe {
			tassert!(crate::memory::NEXT_DMA_VIRT >= 0xFFFF_E000_0000_0000);
			tassert_eq!(crate::memory::NEXT_DMA_VIRT % 4096, 0);
		}
		Ok(())
	}

	pub fn test_boot_info_frame_allocator_deallocate_pushes_to_free_list() -> Result<(), TestError>
	{
		use crate::{arch::x86_64::bootinfo::MemoryMap, memory::BootInfoFrameAllocator};

		unsafe extern "C" {
			static __kernel_phys_start: u8;
			static _end: u8;
		}

		let mut allocator = BootInfoFrameAllocator::init(&DUMMY_MAP);
		let frame = PhysFrame::containing_address(PhysAddr::new(0x3000));
		allocator.deallocate_frame(frame);

		tassert_eq!(allocator.free_list.len(), 1);
		tassert_eq!(allocator.free_list[0], 0x3000);
		Ok(())
	}
	crate::create_test!(test_boot_info_frame_allocator_deallocate_pushes_to_free_list);

	pub fn test_boot_info_frame_allocator_reuses_freed_frames() -> Result<(), TestError> {
		use crate::{arch::x86_64::bootinfo::MemoryMap, memory::BootInfoFrameAllocator};

		let mut allocator = BootInfoFrameAllocator::init(&DUMMY_MAP);
		let frame1 = PhysFrame::containing_address(PhysAddr::new(0x4000));
		let frame2 = PhysFrame::containing_address(PhysAddr::new(0x5000));

		allocator.deallocate_frame(frame2);
		allocator.deallocate_frame(frame1);

		let allocated1 = allocator.allocate_frame().unwrap();
		tassert_eq!(allocated1.start_address().as_u64(), 0x4000);

		let allocated2 = allocator.allocate_frame().unwrap();
		tassert_eq!(allocated2.start_address().as_u64(), 0x5000);

		tassert!(allocator.free_list.is_empty());
		Ok(())
	}
	crate::create_test!(test_boot_info_frame_allocator_reuses_freed_frames);

	pub fn test_is_reserved_kernel_boundaries() -> Result<(), TestError> {
		use crate::{arch::x86_64::bootinfo::MemoryMap, memory::BootInfoFrameAllocator};

		let allocator = BootInfoFrameAllocator::init(&DUMMY_MAP);

		unsafe extern "C" {
			static __kernel_phys_start: u8;
			static _end: u8;
		}

		let kernel_start = core::ptr::addr_of!(__kernel_phys_start) as u64;
		let kernel_end_virt = core::ptr::addr_of!(_end) as u64;
		let kernel_end = if kernel_end_virt >= 0xffff_ffff_8000_0000 {
			kernel_end_virt - 0xffff_ffff_8000_0000
		} else {
			kernel_end_virt
		};

		tassert!(allocator.is_reserved(kernel_start.saturating_sub(1)));
		tassert!(allocator.is_reserved(kernel_start));
		if kernel_end > 0 {
			tassert!(allocator.is_reserved(kernel_end.saturating_sub(1)));
		}

		Ok(())
	}
	crate::create_test!(test_is_reserved_kernel_boundaries);
}
