//!
//! drivers/virtio/mod.rs 
//! 
//! Virtio driver defintions.
//! 

#[allow(unused)]
pub mod net;

use core::{
	ptr::null_mut,
	sync::atomic::{Ordering, fence}
};

use x86_64::{PhysAddr, VirtAddr, align_up};

use crate::{bitflags, common::ports::outw};

const VIRTIO_IO_DEVICE_FEATURES: usize = 0x00;
const VIRTIO_IO_DRIVER_FEATURES: usize = 0x04;
const VIRTIO_IO_QUEUE_ADDR: usize = 0x08; // same as QUEUE_PFN
const VIRTIO_IO_QUEUE_SIZE: usize = 0x0C;
const VIRTIO_IO_QUEUE_SELECT: usize = 0x0E;
const VIRTIO_IO_QUEUE_NOTIFY: usize = 0x10;
/// The Virtio Device Status port
pub const VIRTIO_IO_DEVICE_STATUS: usize = 0x12;
/// The virtio device interrupt service routine port.
pub const VIRTIO_IO_ISR: usize = 0x13;
const VIRTIO_IO_DEVICE_CFG: usize = 0x14; // start of config space

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;

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
/// The virtqueue descriptor.
pub struct VirtqueueDescriptor {
	/// Address of the descriptor.
	pub addr: u64,
	/// Length of the descriptor.
	pub len: u32,
	/// Flags for the descriptor.
	pub flags: u16,
	/// Next descriptor in line.
	pub next: u16
}

#[repr(C)]
#[derive(Default)]
/// The available virtqueue ring.
pub struct VirtqueueAvailable {
	/// Flags
	pub flags: u16,
	/// Index
	pub idx: u16,
	/// Ring
	pub ring: [u16; 0]
}

#[repr(C)]
#[derive(Default)]
/// The used element virtqueue ring.
pub struct VirtqueueUsedElement {
	/// Id
	pub id: u32,
	/// Length of the ring
	pub len: u32
}

#[repr(C)]
#[derive(Default)]
/// The used virtqueue ring.
pub struct VirtqueueUsed {
	/// Flags
	pub flags: u16,
	/// Index
	pub idx: u16,
	/// Ring
	pub ring: [VirtqueueUsedElement; 0]
}

// makes computing the size easier.
// more important stuff like this will get more documentation.
/// Structure representing a VirtQueue.
/// Represents a VirtIO queue used for communication between the driver and device.
/// 
/// A VirtQueue consists of three main components:
/// - Descriptor table: describes memory buffers
/// - Available ring: index of buffers available to the device
/// - Used ring: index of buffers the device has processed
pub struct VirtQueue {
	/// Size of the virtqueue in number of descriptors
	pub size: u16,

	/// Pointer to the descriptor table of the VirtQueue
	pub desc: *mut VirtqueueDescriptor,
	/// Pointer to the available ring of the VirtQueue
	pub avail: *mut VirtqueueAvailable,
	/// Pointer to the used ring of the VirtQueue
	pub used: *mut VirtqueueUsed,

	/// Index of the first free descriptor in the VirtQueue
	pub free_head: u16,
	/// Index of the last descriptor processed by the device
	pub last_used: u16,

	/// Number of free descriptors available in the VirtQueue
	pub num_free: u16,

	/// Physical address of the VirtQueue
	pub phys_addr: PhysAddr,
	/// Virtual address of the VirtQueue
	pub virt_addr: VirtAddr,

	/// Index identifying this VirtQueue for the device
	pub queue_index: u16,
	/// I/O base address for device communication
	pub io_base: u16
}

unsafe impl Send for VirtQueue {}
unsafe impl Sync for VirtQueue {}

impl VirtQueue {
	/// Creates an empty `VirtQueue` with no values inside.
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
	fn init_free_list(&mut self) {
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

	fn add_descriptor(
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

	fn free_descriptor(&mut self, desc_idx: u16) {
		unsafe {
			let desc = &mut *self.desc.add(desc_idx as usize);
			desc.next = self.free_head;
		}
		self.free_head = desc_idx;
		self.num_free += 1;
	}

	fn push_avail(&mut self, desc_index: u16) {
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

	fn kick(&self) {
		unsafe {
			outw(
				self.io_base + VIRTIO_IO_QUEUE_NOTIFY as u16,
				self.queue_index
			);
		}
	}

	fn pop_used(&mut self) -> Option<(u16, u32)> {
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

fn virtqueue_size(qsize: usize) -> usize {
	let desc_size = qsize * core::mem::size_of::<VirtqueueDescriptor>();
	let avail_size =
		core::mem::size_of::<VirtqueueAvailable>() + qsize * core::mem::size_of::<u16>();

	let used_size = core::mem::size_of::<VirtqueueUsed>()
		+ qsize * core::mem::size_of::<VirtqueueUsedElement>();

	let used_offset = align_up((desc_size + avail_size).try_into().unwrap(), 4096);
	(used_offset + used_size as u64).try_into().unwrap()
}

/// Trait for all Virtio Devices to implement.
pub trait VirtioDevice {
	/// Get and return the current negotiated device features.
	fn device_features(&mut self) -> u64;
	/// Set driver features.
	fn set_driver_features(&mut self, features: u64);
	/// Allocate a Virtqueue for the device.
	fn alloc_virtqueue(&mut self, virtq: u16) -> Result<VirtQueue, &'static str>;
	/// Get the current driver status.
	fn driver_status(&mut self) -> u16;
	/// Set the current driver status.
	fn set_driver_status(&mut self, status: u8);
	/// If the driver is set to a current status.
	fn has_status(&mut self, status: u8) -> bool;
	/// All currently supported features
	// same as device_features()?
	fn supported_features(&mut self) -> u64;

	/// Initialise the VirtIO device.
	fn init(&mut self) -> Result<(), &'static str>;
}
