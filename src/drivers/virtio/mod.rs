pub mod net;

use core::{
	ptr::null_mut,
	sync::atomic::{Ordering, fence}
};

use x86_64::{PhysAddr, VirtAddr, align_up};

use crate::{bitflags, common::ports::outw};

pub const VIRTIO_IO_DEVICE_FEATURES: usize = 0x00;
pub const VIRTIO_IO_DRIVER_FEATURES: usize = 0x04;
pub const VIRTIO_IO_QUEUE_ADDR: usize = 0x08; // same as QUEUE_PFN
pub const VIRTIO_IO_QUEUE_SIZE: usize = 0x0C;
pub const VIRTIO_IO_QUEUE_SELECT: usize = 0x0E;
pub const VIRTIO_IO_QUEUE_NOTIFY: usize = 0x10;
pub const VIRTIO_IO_DEVICE_STATUS: usize = 0x12;
pub const VIRTIO_IO_ISR: usize = 0x13;
pub const VIRTIO_IO_DEVICE_CFG: usize = 0x14; // start of config space

pub const VIRTQ_DESC_F_NEXT: u16 = 1;
pub const VIRTQ_DESC_F_WRITE: u16 = 2;

bitflags! {
	/// A simple low-level indication of the completed steps in the device
	/// initialisation.<br>
	/// ## Driver Requirements<br>
	/// The driver MUST update device status, setting bits to indicate the completed steps
	/// of the driver initialization sequence.<br>
	///
	/// The driver MUST NOT clear a device status bit. If the driver sets the `FAILED` bit
	/// the driver MUST later reset the device before attempting to re-initialize<br>
	///
	/// The driver SHOULD NOT rely on completion of operations of a device if DEVICE_NEEDS_RESET
	/// is set.
	///
	/// ## Device Requirements<br>
	/// The device MUST NOT consume buffers or send any used buffer notifications to the driver
	/// before `DRIVER_OK`.<br>
	///
	/// The device SHOULD set `DEVICE_NEEDS_RESET` when it enters an error state that a reset is needed.
	/// If `DRIVER_OK` is set, after it sets `DEVICE_NEEDS_RESET`, the device MUST send a device configuration
	/// change notification to the driver.
	pub struct VirtIODeviceStatus: u8 {
		const ZERO = 0;

		/// Indicates that the guest OS has found the device and recognised it as a
		/// valid virtio device.
		const ACKNOWLEDGE = 1 << 0;

		/// Indicates that the guest OS knows how to drive the device.
		const DRIVER = 1 << 1;

		/// Indicates that the driver is set up and ready to drive the device.
		const DRIVER_OK = 1 << 2;

		/// Indicates that the driver has acknowledged all the features it
		/// understand, and feature negotiation is complete.
		const FEATURES_OK = 1 << 3;

		/// Indicates that the device has experienced an error from which it
		/// can't recover.
		const DEVICE_NEEDS_RESET = 1 << 6;

		/// Indicates that something went wrong in the guest, and it has given
		/// up on the device.
		const FAILED = 1 << 7;

		/// incase for C АBI
		const _ = !0;
	}
}

// based off https://github.com/rust-vmm/vm-virtio/blob/main/virtio-bindings/src/bindings/x86_64/virtio_ring.rs
// and https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html#x1-490006
#[repr(C)]
#[derive(Copy, Clone)]
pub struct VirtqueueDescriptor {
	pub addr: u64,
	pub len: u32,
	pub flags: u16,
	pub next: u16
}

#[repr(C)]
#[derive(Default)]
pub struct VirtqueueAvailable {
	pub flags: u16,
	pub idx: u16,
	pub ring: [u16; 0]
}

#[repr(C)]
#[derive(Default)]
pub struct VirtqueueUsedElement {
	pub id: u32,
	pub len: u32
}

#[repr(C)]
#[derive(Default)]
pub struct VirtqueueUsed {
	pub flags: u16,
	pub idx: u16,
	pub ring: [VirtqueueUsedElement; 0]
}

// makes computing the size easier.
pub struct VirtQueue {
	pub size: u16,

	pub desc: *mut VirtqueueDescriptor,
	pub avail: *mut VirtqueueAvailable,
	pub used: *mut VirtqueueUsed,

	pub free_head: u16,
	pub last_used: u16,

	// Add free list tracking
	pub num_free: u16,

	pub phys_addr: PhysAddr,
	pub virt_addr: VirtAddr,

	pub queue_index: u16,
	pub io_base: u16
}

unsafe impl Send for VirtQueue {}
unsafe impl Sync for VirtQueue {}

impl VirtQueue {
	pub fn empty() -> VirtQueue {
		VirtQueue {
			size: 0,
			desc: null_mut(),
			avail: null_mut(),
			used: null_mut(),
			free_head: 0,
			last_used: 0,
			num_free: 0,
			phys_addr: PhysAddr::zero(),
			virt_addr: VirtAddr::zero(),
			queue_index: 0,
			io_base: 0
		}
	}

	// Initialize the free list after allocation
	pub fn init_free_list(&mut self) {
		self.num_free = self.size;
		self.free_head = 0;

		unsafe {
			for i in 0..self.size {
				let desc = &mut *self.desc.add(i as usize);
				desc.next = if i + 1 < self.size { i + 1 } else { 0 };
				desc.flags = 0;
			}
		}
	}

	pub fn add_descriptor(
		&mut self,
		phys_addr: PhysAddr,
		len: u32,
		device_writes: bool
	) -> Result<u16, &'static str> {
		if self.num_free == 0 {
			return Err("virtqueue full");
		}

		let idx = self.free_head;

		unsafe {
			let desc = &mut *self.desc.add(idx as usize);
			self.free_head = desc.next;

			desc.addr = phys_addr.as_u64();
			desc.len = len;
			desc.flags = if device_writes { VIRTQ_DESC_F_WRITE } else { 0 };
			desc.next = 0;
		}

		self.num_free -= 1;
		Ok(idx)
	}

	pub fn free_descriptor(&mut self, desc_idx: u16) {
		unsafe {
			let desc = &mut *self.desc.add(desc_idx as usize);
			desc.next = self.free_head;
		}
		self.free_head = desc_idx;
		self.num_free += 1;
	}

	pub fn push_avail(&mut self, desc_index: u16) {
		let avail = unsafe { &mut *self.avail };
		let ring_ptr = unsafe {
			(avail as *mut _ as *mut u8)
				.add(core::mem::size_of::<VirtqueueAvailable>())
				.add((avail.idx % self.size) as usize * 2) as *mut u16
		};
		unsafe { ring_ptr.write(desc_index) };
		fence(Ordering::Release);
		avail.idx = avail.idx.wrapping_add(1);
	}

	pub fn kick(&self) {
		unsafe {
			outw(
				self.io_base + VIRTIO_IO_QUEUE_NOTIFY as u16,
				self.queue_index
			);
		}
	}

	pub fn pop_used(&mut self) -> Option<(u16, u32)> {
		let used = unsafe { &*self.used };

		fence(Ordering::Acquire);

		if self.last_used == used.idx {
			return None;
		}

		let index = (self.last_used % self.size) as usize;

		let elem = unsafe {
			let ring = (used as *const _ as *const u8).add(core::mem::size_of::<VirtqueueUsed>())
				as *const VirtqueueUsedElement;

			&*ring.add(index)
		};

		self.last_used = self.last_used.wrapping_add(1);

		Some((elem.id as u16, elem.len))
	}
}

pub fn virtqueue_size(qsize: usize) -> usize {
	let desc_size = qsize * core::mem::size_of::<VirtqueueDescriptor>();
	let avail_size =
		core::mem::size_of::<VirtqueueAvailable>() + qsize * core::mem::size_of::<u16>();

	let used_size = core::mem::size_of::<VirtqueueUsed>()
		+ qsize * core::mem::size_of::<VirtqueueUsedElement>();

	let used_offset = align_up((desc_size + avail_size).try_into().unwrap(), 4096);
	(used_offset + used_size as u64).try_into().unwrap()
}

pub trait VirtioDevice {
	fn device_features(&mut self) -> u64;
	fn set_driver_features(&mut self, features: u64);
	fn alloc_virtqueue(&mut self, virtq: u16) -> Result<VirtQueue, &'static str>;
	fn driver_status(&mut self) -> u16;
	fn set_driver_status(&mut self, status: u8);
	fn has_status(&mut self, status: u8) -> bool;
	fn supported_features(&mut self) -> u64;

	fn init(&mut self) -> Result<(), &'static str>;
}
