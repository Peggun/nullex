// code from this great website
// https://nfil.dev/kernel/rust/coding/rust-buddy-allocator/

use alloc::vec::Vec;
use core::{alloc::GlobalAlloc, cmp};

use x86_64::{
	PhysAddr,
	VirtAddr,
	structures::paging::{FrameAllocator, Size4KiB}
};

use crate::{
	memory::{phys_to_virt, virt_to_phys},
	serial_println,
	utils::{mutex::SpinMutex, spin::rwlock::RwLock}
};

pub enum BuddyMemoryAreaRequest {
	Success((PhysAddr, PhysAddr)),
	SmallerThanReq((PhysAddr, PhysAddr), Option<(PhysAddr, PhysAddr)>),
	Fail
}

#[derive(Debug)]
pub struct BuddyAllocator {
	pub start_addr: PhysAddr,
	pub end_addr: PhysAddr,
	pub num_levels: u8,
	pub block_size: u16,
	pub free_lists: Vec<Vec<u32>>
}

impl BuddyAllocator {
	pub fn max_size(&self) -> usize {
		(self.block_size as usize) << (self.num_levels as usize)
	}

	pub fn new(start_addr: PhysAddr, end_addr: PhysAddr, block_size: u16) -> BuddyAllocator {
		let mut num_levels = 0;
		while ((block_size as u64) << block_size as u64) < end_addr.as_u64() - start_addr.as_u64() {
			num_levels += 1;
		}

		let mut free_lists = Vec::with_capacity((num_levels + 1) as usize);
		for _ in 0..(num_levels + 1) {
			free_lists.push(Vec::with_capacity(4));
		}

		free_lists[0].push(0);

		BuddyAllocator {
			start_addr,
			end_addr,
			num_levels,
			block_size,
			free_lists
		}
	}

	pub fn contains(&self, addr: PhysAddr) -> bool {
		addr.as_u64() >= self.start_addr.as_u64() && addr.as_u64() < self.end_addr.as_u64()
	}

	pub fn req_size_to_level(&self, size: usize) -> Option<usize> {
		let max_size = self.max_size();
		if size > max_size {
			None
		} else {
			let mut next_level = 1;
			while (max_size >> next_level) >= size {
				next_level += 1
			}
			let req_level = cmp::min(next_level - 1, self.num_levels as usize);
			Some(req_level)
		}
	}

	pub fn get_free_block(&mut self, level: usize) -> Option<u32> {
		self.free_lists[level]
			.pop()
			.or_else(|| self.split_level(level))
	}

	pub fn split_level(&mut self, level: usize) -> Option<u32> {
		if level == 0 {
			None
		} else {
			self.get_free_block(level - 1).map(|block| {
				self.free_lists[level].push(block * 2 + 1);
				block * 2
			})
		}
	}

	pub fn merge_buddies(&mut self, level: usize, block_num: u32) {
		let buddy_block = block_num ^ 1;
		if let Some(buddy_idx) = self.free_lists[level]
			.iter()
			.position(|blk| *blk == buddy_block)
		{
			self.free_lists[level].pop();
			self.free_lists[level].remove(buddy_idx);
			self.free_lists[level - 1].push(block_num / 2);
			self.merge_buddies(level - 1, block_num / 2);
		}
	}

	pub fn alloc(&mut self, size: usize, alignment: usize) -> Option<PhysAddr> {
		let size = cmp::max(size, alignment);
		self.req_size_to_level(size).and_then(|req_level| {
			self.get_free_block(req_level).map(|block| {
				let offset = block as u64 * (self.max_size() >> req_level as usize) as u64;
				PhysAddr::new(self.start_addr.as_u64() + offset)
			})
		})
	}

	pub fn dealloc(&mut self, addr: PhysAddr, size: usize, alignment: usize) {
		let size = cmp::max(size, alignment);

		if let Some(req_level) = self.req_size_to_level(size) {
			let level_block_size = self.max_size() >> req_level;
			let block_num =
				((addr.as_u64() - self.start_addr.as_u64()) as usize / level_block_size) as u32;
			self.free_lists[req_level].push(block_num);
			self.merge_buddies(req_level, block_num);
		}
	}
}

pub struct BuddyAllocatorManager {
	pub buddy_allocators: RwLock<Vec<SpinMutex<BuddyAllocator>>>
}

impl BuddyAllocatorManager {
	pub fn new() -> BuddyAllocatorManager {
		let buddy_allocators = RwLock::new(Vec::with_capacity(32));
		BuddyAllocatorManager {
			buddy_allocators
		}
	}

	pub fn add_memory_area(&self, start_addr: PhysAddr, end_addr: PhysAddr, block_size: u16) {
		let new_buddy_alloc = SpinMutex::new(BuddyAllocator::new(start_addr, end_addr, block_size));
		self.buddy_allocators.write().push(new_buddy_alloc);
	}

	pub fn add_memory_area_with_size(
		&self,
		frame_alloc: &mut impl FrameAllocator<Size4KiB>,
		mem_size: u64,
		block_size: u16
	) -> bool {
		loop {
			match Self::get_memory_area_with_size(frame_alloc, mem_size) {
				BuddyMemoryAreaRequest::Success((mem_start, mem_end)) => {
					serial_println!(
						"* Adding requested mem area to BuddyAlloc: {:?} to {:?} ({})",
						mem_start,
						mem_end,
						mem_end.as_u64() - mem_start.as_u64()
					);

					self.add_memory_area(mem_start, mem_end, block_size);
					return true;
				}
				BuddyMemoryAreaRequest::SmallerThanReq((mem_start, mem_end), second_area) => {
					self.add_memory_area(mem_start, mem_end, block_size);
					serial_println!(
						"* Adding smaller mem area to BuddyAlloc: {:?} to {:?} ({})",
						mem_start,
						mem_end,
						mem_end.as_u64() - mem_start.as_u64()
					);
					if let Some((mem_start, mem_end)) = second_area {
						self.add_memory_area(mem_start, mem_end, block_size);
						serial_println!(
							"* Adding smaller mem area to BuddyAlloc: {:?} to {:?} ({})",
							mem_start,
							mem_end,
							mem_end.as_u64() - mem_start.as_u64()
						);
					}
				}
				BuddyMemoryAreaRequest::Fail => {
					serial_println!(
						"! Failed to find mem area big enough for BuddyAlloc: {}",
						mem_size
					);
					return false;
				}
			}
		}
	}

	pub fn get_memory_area_with_size(
		frame_alloc: &mut impl FrameAllocator<Size4KiB>,
		mem_size: u64
	) -> BuddyMemoryAreaRequest {
		if let Some(first_page) = frame_alloc.allocate_frame() {
			let first_addr = first_page.start_address().as_u64();
			let mut last_addr = first_addr + 4096;
			while let Some(next_page) = frame_alloc.allocate_frame() {
				if next_page.start_address().as_u64() == last_addr {
					last_addr += 4096;
				} else {
					break
				}
				if last_addr - first_addr == mem_size {
					break;
				}
			}

			if last_addr - first_addr == mem_size {
				BuddyMemoryAreaRequest::Success((
					PhysAddr::new(first_addr),
					PhysAddr::new(last_addr)
				))
			} else {
				if let Some(first_memarea) = Self::get_largest_page_multiple(first_addr, last_addr)
				{
					let second_memarea =
						Self::get_largest_page_multiple(first_memarea.1.as_u64(), last_addr);
					BuddyMemoryAreaRequest::SmallerThanReq(first_memarea, second_memarea)
				} else {
					BuddyMemoryAreaRequest::Fail
				}
			}
		} else {
			BuddyMemoryAreaRequest::Fail
		}
	}

	pub fn get_largest_page_multiple(start: u64, end: u64) -> Option<(PhysAddr, PhysAddr)> {
		let mem_len = end - start;
		if mem_len == 0 {
			None
		} else {
			let mut page_mult = 4096;
			while page_mult <= mem_len {
				page_mult <<= 1;
			}
			page_mult >>= 1;
			let start_addr = PhysAddr::new(start);
			Some((start_addr, PhysAddr::new(start_addr.as_u64() + page_mult)))
		}
	}
}

unsafe impl GlobalAlloc for BuddyAllocatorManager {
	unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
		let allocation =
			self.buddy_allocators
				.read()
				.iter()
				.enumerate()
				.find_map(|(i, allocator)| {
					allocator.try_lock().and_then(|mut allocator| {
						allocator
							.alloc(layout.size(), layout.align())
							.map(|allocation| {
								serial_println!(
									" - BuddyAllocator #{} allocated {} bytes",
									i,
									layout.size()
								);
								serial_println!("{:?}", *allocator);
								allocation
							})
					})
				});
		unsafe { phys_to_virt(allocation.unwrap()).as_mut_ptr() }
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
		let virt_addr = VirtAddr::new(ptr as u64);
		if let Some(phys_addr) = unsafe { virt_to_phys(virt_addr) } {
			for (i, allocator_mtx) in self.buddy_allocators.read().iter().enumerate() {
				if let Some(mut allocator) = allocator_mtx.try_lock() {
					if allocator.contains(phys_addr) {
						allocator.dealloc(phys_addr, layout.size(), layout.align());

						serial_println!(
							" - BuddyAllocator #{} de-allocated {} bytes",
							i,
							layout.size()
						);
						serial_println!("{:?}", *allocator);
						return;
					}
				}
			}
		}

		serial_println!(
			"! Could not de-allocate virtual address: {:?} / Memory lost",
			virt_addr
		);
	}
}
