use alloc::vec::Vec;

use x86_64::{
	PhysAddr,
	VirtAddr,
	registers::control::{Cr3, Cr3Flags},
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
	allocator::ALLOCATOR_INFO,
	arch::x86_64::bootinfo::MemoryRegion,
	error::NullexError,
	gdt::{
		DOUBLE_FAULT_STACK_SIZE,
		INTERRUPT_STACK_SIZE,
		double_fault_stack_top,
		interrupt_stack_top
	},
	memory::{PHYS_MEM_OFFSET, active_level_4_table, phys_to_virt},
	serial_println,
	task::KERNEL_CR3
};

/// Structure representing the memory region each `Process` has.
pub struct AddressSpace {
	/// Physical frame of the memory region.
	pub page_table: PhysFrame,
	/// Regions of the memory.
	pub regions: Vec<MemoryRegion>,
	pub kernel_pml4_indices: Vec<usize>,
	pub owned_frames: Vec<PhysFrame>
}

impl AddressSpace {
	pub fn new() -> Result<AddressSpace, NullexError> {
		let mut frame_binding = ALLOCATOR_INFO.frame_allocator.lock();
		let frame_allocator = frame_binding
			.as_mut()
			.ok_or(NullexError::FrameAllocatorNotInitialized)?;

		let pml4_frame = frame_allocator
			.allocate_frame()
			.ok_or(NullexError::FrameAllocationFailed)?;

		let table_ptr = unsafe { phys_to_virt(pml4_frame.start_address()) };
		unsafe {
			core::ptr::write_bytes(table_ptr.as_mut_ptr::<u8>(), 0, 4096);
		}

		let phys_offset = VirtAddr::new(PHYS_MEM_OFFSET);
		let kernel_pml4 = unsafe { active_level_4_table(phys_offset) };
		let new_pml4 = unsafe { &mut *table_ptr.as_mut_ptr::<PageTable>() };

		let mut kernel_pml4_indices = Vec::new();

		// clone all present kernel PML4 entries
		for i in 256..512 {
			if kernel_pml4[i].flags().contains(PageTableFlags::PRESENT) {
				new_pml4[i] = kernel_pml4[i].clone();
				kernel_pml4_indices.push(i);
			}
		}

		let mut new_mapper = unsafe { OffsetPageTable::new(new_pml4, phys_offset) };

		// Map the interrupt stack into the new address space
		let mut map_page = |virt: u64| -> Result<(), NullexError> {
			let page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(virt));

			// Because we cloned all kernel mappings, the interrupt stack
			// is likely already mapped! This check prevents redundant allocations.
			if new_mapper.translate_addr(page.start_address()).is_some() {
				return Ok(());
			}

			let old_mapper = unsafe { OffsetPageTable::new(kernel_pml4, phys_offset) };
			let phys = match old_mapper.translate_addr(page.start_address()) {
				Some(phys) => phys,
				None => return Ok(())
			};
			let frame = PhysFrame::containing_address(phys);
			unsafe {
				new_mapper
					.map_to(
						page,
						frame,
						PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
						&mut **frame_allocator
					)
					.map_err(|err| {
						serial_println!("map_to failed for virt {:#x}: {:?}", virt, err);
						NullexError::FrameAllocationFailed
					})?
					.flush();
			}
			Ok(())
		};

		let int_stack_top = interrupt_stack_top();
		for i in 0..(INTERRUPT_STACK_SIZE / 4096) {
			let addr = int_stack_top - 1 - (i as u64 * 0x1000);
			map_page(addr)?;
		}

		let double_fault_ist_index = double_fault_stack_top();
		for i in 0..(DOUBLE_FAULT_STACK_SIZE / 4096) {
			let addr = double_fault_ist_index - 1 - (i as u64 * 0x1000);
			map_page(addr)?;
		}

		Ok(AddressSpace {
			page_table: pml4_frame,
			regions: Vec::new(),
			kernel_pml4_indices,
			owned_frames: Vec::new()
		})
	}
}

impl Drop for AddressSpace {
	fn drop(&mut self) {
		let _kg = unsafe { Cr3Guard::enter_kernel() }; // walk via the kernel direct-map
		let mut frame_binding = crate::allocator::ALLOCATOR_INFO.frame_allocator.lock();
		let Some(fa) = frame_binding.as_mut() else {
			return;
		};

		// DEBUG: track freed frames to catch double-frees
		#[cfg(debug_assertions)]
		let mut freed_frames = alloc::collections::BTreeSet::new();

		// free leaf data/stack frames
		for frame in self.owned_frames.drain(..) {
			#[cfg(debug_assertions)]
			if !freed_frames.insert(frame.start_address().as_u64()) {
				serial_println!(
					"[FATAL] Double free detected in owned_frames: {:#x}",
					frame.start_address().as_u64()
				);
				crate::hlt_loop();
			}
			fa.deallocate_frame(frame);
		}

		// walk and free intermediate page tables
		let pml4_virt = unsafe { crate::memory::phys_to_virt(self.page_table.start_address()) };
		let pml4 = unsafe { &*pml4_virt.as_ptr::<PageTable>() };

		for (i, entry) in pml4.iter().enumerate() {
			if self.kernel_pml4_indices.contains(&i) {
				continue;
			}
			let f = entry.flags();
			if !f.contains(PageTableFlags::PRESENT) {
				continue;
			}
			if f.contains(PageTableFlags::HUGE_PAGE) {
				continue;
			}

			let Ok(pml3_frame) = entry.frame() else {
				continue;
			};
			#[cfg(debug_assertions)]
			if !freed_frames.insert(pml3_frame.start_address().as_u64()) {
				serial_println!(
					"[FATAL] Double free detected in PML3: {:#x}",
					pml3_frame.start_address().as_u64()
				);
				crate::hlt_loop();
			}

			let pml3 = unsafe {
				&*crate::memory::phys_to_virt(pml3_frame.start_address()).as_ptr::<PageTable>()
			};
			for entry in pml3.iter() {
				let f = entry.flags();
				if !f.contains(PageTableFlags::PRESENT) {
					continue;
				}
				if f.contains(PageTableFlags::HUGE_PAGE) {
					continue;
				}

				let Ok(pml2_frame) = entry.frame() else {
					continue;
				};
				#[cfg(debug_assertions)]
				if !freed_frames.insert(pml2_frame.start_address().as_u64()) {
					serial_println!(
						"[FATAL] Double free detected in PML2: {:#x}",
						pml2_frame.start_address().as_u64()
					);
					crate::hlt_loop();
				}

				let pml2 = unsafe {
					&*crate::memory::phys_to_virt(pml2_frame.start_address()).as_ptr::<PageTable>()
				};
				for entry in pml2.iter() {
					let f = entry.flags();
					if !f.contains(PageTableFlags::PRESENT) {
						continue;
					}
					if f.contains(PageTableFlags::HUGE_PAGE) {
						continue;
					}

					let Ok(pml1_frame) = entry.frame() else {
						continue;
					};
					#[cfg(debug_assertions)]
					if !freed_frames.insert(pml1_frame.start_address().as_u64()) {
						serial_println!(
							"[FATAL] Double free detected in PML1: {:#x}",
							pml1_frame.start_address().as_u64()
						);
						crate::hlt_loop();
					}
					fa.deallocate_frame(pml1_frame);
				}
				fa.deallocate_frame(pml2_frame);
			}
			fa.deallocate_frame(pml3_frame);
		}

		// free the PML4 frame itself
		#[cfg(debug_assertions)]
		if !freed_frames.insert(self.page_table.start_address().as_u64()) {
			serial_println!(
				"[FATAL] Double free detected in PML4: {:#x}",
				self.page_table.start_address().as_u64()
			);
			crate::hlt_loop();
		}
		fa.deallocate_frame(self.page_table);

		x86_64::instructions::tlb::flush_all();
	}
}

pub struct Cr3Guard {
	pub previous: (PhysFrame, Cr3Flags),
	pub switched: bool
}

impl Cr3Guard {
	pub unsafe fn enter_kernel() -> Self {
		let previous = Cr3::read();
		let kernel_cr3 = unsafe { KERNEL_CR3 };
		let switched = kernel_cr3 != 0 && previous.0.start_address().as_u64() != kernel_cr3;

		if switched {
			unsafe {
				Cr3::write(
					PhysFrame::containing_address(PhysAddr::new(kernel_cr3)),
					previous.1
				);
			}
		}

		Cr3Guard {
			previous,
			switched
		}
	}
}

impl Drop for Cr3Guard {
	fn drop(&mut self) {
		if self.switched {
			unsafe {
				Cr3::write(self.previous.0, self.previous.1);
			}
		}
	}
}
