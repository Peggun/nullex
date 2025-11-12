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
		Size4KiB,
		Translate
	}
};

use crate::{println, serial_println, utils::multiboot2::{__link_phys_base, _end}};

pub fn map_apic(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    physical_memory_offset: VirtAddr,
) {
    println!("[Info] Mapping APIC Timer...");

    const APIC_PHYS_START: u64 = 0xFEE0_0000u64;
    let apic_phys = PhysAddr::new(APIC_PHYS_START);
    let apic_frame = PhysFrame::containing_address(apic_phys);

    // compute the virtual address we actually use to access physical memory
    let apic_virt = VirtAddr::new(physical_memory_offset.as_u64() + APIC_PHYS_START);
    let apic_page = Page::containing_address(apic_virt);

    let apic_flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::NO_CACHE;

    unsafe {
        mapper
            .map_to(apic_page, apic_frame, apic_flags, frame_allocator)
            .unwrap()
            .flush();
    }

    println!("[Info] Done.");
}

// map_ioapic in memory.rs (patterned after your map_apic)
pub fn map_ioapic(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    physical_memory_offset: VirtAddr,
) {
    println!("[Info] Mapping IOAPIC...");

    const IOAPIC_PHYS_START: u64 = 0xFEC0_0000u64;
    let ioapic_phys = PhysAddr::new(IOAPIC_PHYS_START);
    let ioapic_frame = PhysFrame::containing_address(ioapic_phys);

    // virtual address that maps to the physical IOAPIC
    let ioapic_virt = VirtAddr::new(physical_memory_offset.as_u64() + IOAPIC_PHYS_START);
    let ioapic_page = Page::containing_address(ioapic_virt);

    let ioapic_flags = PageTableFlags::PRESENT
        | PageTableFlags::WRITABLE
        | PageTableFlags::NO_CACHE;

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
        let kernel_end   = unsafe { &_end as *const _ as u64 };

        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());

        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));

        frame_addresses
            .filter(move |addr| (addr < &kernel_start) || (addr >= &kernel_end))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
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
/// # Safety
/// We need all memory mapped at `physical_memory_offset`.
pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
	unsafe { translate_addr_inner(addr, physical_memory_offset) }
}

/// function that is called by `translate_addr`.
unsafe fn translate_addr_inner(
	addr: VirtAddr,
	physical_memory_offset: VirtAddr
) -> Option<PhysAddr> {
	let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };
	unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }.translate_addr(addr)
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

/// # Safety
/// We need all memory mapped at `physical_memory_offset`.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
	let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };
	unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) }
}