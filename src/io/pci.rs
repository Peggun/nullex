//!
//! pci.rs
//! 
//! PCI device handling logic for the kernel.
//! 

use alloc::vec::Vec;

use crate::{
	allocator::io_alloc::IO_ALLOC,
	common::ports::{inl, outb, outl, outq, outw},
	lazy_static,
	serial_println,
	utils::{
		mutex::SpinMutex,
		types::{DWORD, WORD}
	}
};

/// Virtio PCI Vendor ID
pub const VIRTIO_PCI_VENDOR_ID: u16 = 0x1af4;
/// Intel PCI Vendor ID
pub const INTEL_VENDOR_ID: u16 = 0x8086;

const PCI_COMMAND_IO: u16 = 0x0001;
const PCI_BUS_MASTER: u16 = 0x0004;

const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

lazy_static! {
	/// List of all current Pci Devices
	pub static ref PCI_DEVICES: SpinMutex<Vec<PciDevice>> = SpinMutex::new(Vec::with_capacity(32));
	/// List of all the drivers information.
	pub static ref DRIVER_TABLE: SpinMutex<Vec<DriverInfo>> = SpinMutex::new(Vec::new());
}

#[derive(Debug, Clone, Copy)]
/// Structure representing the bus number, device number and function number of a PCI device.
pub struct Bdf {
	/// The bus of the PCI device.
	pub bus: u8,
	/// The device of the PCI device.
	pub device: u8,
	/// The function of the PCI device.
	pub func: u8
}

impl Bdf {
	/// Creates a new `Bdf` with the specified bus, device and function.
	pub fn new(bus: u8, device: u8, func: u8) -> Bdf {
		Self {
			bus,
			device,
			func
		}
	}
}

/// Callback type for finalizing device initialization after IOAPIC setup
pub type DeviceFinalizeCallback = fn() -> Result<(), &'static str>;

/// Representation of a discovered PCI Device
#[allow(dead_code)]
pub struct PciDevice {
	/// The Bus, Device and Function of the device.
	pub bdf: Bdf,
	info: DriverInfo,
	bound_driver: Option<usize>,
	// to be added.
	mmio_base: Option<usize>,
	/// The Base IO address for the device.
	pub io_base: Option<usize>,
	io_size: Option<usize>,

	finalize_callback: Option<DeviceFinalizeCallback>
}

impl PciDevice {
	/// Creates a new `PciDevice` with no checks.
	pub fn new_raw(
		bdf: Bdf,
		info: DriverInfo,
		bound_driver: Option<usize>,
		mmio_base: Option<usize>,
		io_base: Option<usize>,
		io_size: Option<usize>
	) -> Self {
		Self {
			bdf,
			info,
			bound_driver,
			mmio_base,
			io_base,
			io_size,
			finalize_callback: None
		}
	}

	/// Get the interrupt line from this device.
	pub fn interrupt_line(&self) -> u8 {
		pci_config_read::<u8>(self.bdf, 0x3C).unwrap()
	}

	/// Set the finalize callback for this device
	pub fn set_finalize_callback(&mut self, callback: DeviceFinalizeCallback) {
		self.finalize_callback = Some(callback);
	}
}

#[derive(Debug, Clone, Copy)]
/// Structure representing all information about the driver running a PCI device.
pub struct DriverInfo {
	/// Vendor of the driver
	pub vendor: Option<u16>,
	/// The device which the driver is driving.
	pub device: Option<u16>,
	/// The class of the driver
	pub class: Option<u8>,
	/// The subclass of the driver
	pub subclass: Option<u8>,
	/// The function which probes and ebales the PCI device.
	pub probe: Option<fn(&mut PciDevice) -> Result<usize, &'static str>>
}

/// Registers a drvier to the driver table.
pub fn register_driver(info: DriverInfo) {
	let mut dt = DRIVER_TABLE.lock();
	dt.push(info);
	serial_println!(
		"[PCI] Registered driver: vendor={:?}, device={:?}, class={:?}",
		info.vendor,
		info.device,
		info.class
	);
}

/// Finalize all PCI devices.
pub fn finalize_all_devices() -> Result<(), &'static str> {
	serial_println!("[PCI] Finalizing all devices with pending callbacks...");

	let callbacks: Vec<DeviceFinalizeCallback> = {
		let devices = PCI_DEVICES.lock();
		devices
			.iter()
			.filter_map(|dev| dev.finalize_callback)
			.collect()
	};

	let count = callbacks.len();
	serial_println!("[PCI] Found {} devices to finalize", count);

	for (idx, callback) in callbacks.iter().enumerate() {
		serial_println!("[PCI] Finalizing device {}/{}", idx + 1, count);
		callback()?;
	}

	serial_println!("[PCI] All {} devices finalized successfully", count);
	Ok(())
}

/// Addds a PCI device.
pub fn add_pci_device(dev: PciDevice) -> usize {
	let mut devices = PCI_DEVICES.lock();
	let idx = devices.len();
	devices.push(dev);
	idx
}

fn matches(info: &DriverInfo, dev: &PciDevice) -> bool {
	if let Some(v) = info.vendor {
		if v != dev.info.vendor.expect("no vendor") {
			return false;
		}
	}
	if let Some(d) = info.device {
		if d != dev.info.device.expect("no device id") {
			return false;
		}
	}
	if let Some(c) = info.class {
		if c != dev.info.class.expect("no device class") {
			return false;
		}
	}
	if let Some(s) = info.subclass {
		if s != dev.info.subclass.expect("no device subclass") {
			return false;
		}
	}
	true
}

/// Read `N` type from the PCI Config. Assuming N is a unsigned integer.
pub fn pci_config_read<N>(bdf: Bdf, offset: u8) -> Result<N, <N as TryFrom<u64>>::Error>
where
	N: TryFrom<u64> + Copy
{
	let lbus = bdf.bus as u32;
	let lslot = bdf.device as u32;
	let lfunc = bdf.func as u32;
	let address =
		(lbus << 16) | (lslot << 11) | (lfunc << 8) | ((offset as u32) & 0xFC) | 0x8000_0000u32;

	unsafe { outl(PCI_CONFIG_ADDRESS, address) };

	let data = unsafe { inl(PCI_CONFIG_DATA) } as u64;

	let shift = ((offset as u64) & 3) * 8;
	let bits = (size_of::<N>() * 8) as u64;
	let mask = if bits == 64 {
		!0u64
	} else {
		(1u64 << bits) - 1u64
	};

	let val = (data >> shift) & mask;

	N::try_from(val)
}

/// Write `N` type to the PCI Config. Assuming N is a unsigned integer.
pub fn pci_config_write<N>(bdf: Bdf, offset: u8, value: N) -> Result<(), &'static str>
where
	N: Into<u64> + Copy
{
	let lbus = bdf.bus as u32;
	let ldev = bdf.device as u32;
	let lfunc = bdf.func as u32;
	let address =
		(lbus << 16) | (ldev << 11) | (lfunc << 8) | ((offset as u32) & 0xFC) | 0x8000_0000u32;

	let val = value.into();

	unsafe {
		outl(PCI_CONFIG_ADDRESS, address);

		if size_of::<N>() == 1 {
			outb(PCI_CONFIG_DATA, val as u8);
		} else if size_of::<N>() == 2 {
			outw(PCI_CONFIG_DATA, val as u16);
		} else if size_of::<N>() == 4 {
			outl(PCI_CONFIG_DATA, val as u32);
		} else {
			outq(PCI_CONFIG_DATA, val);
		}
	}

	Ok(())
}

/// Discover all PCI devices currently connected.
pub fn discover_pci_devices() {
	serial_println!("[PCI] Starting PCI device discovery...");

	for bus in 0..=255 {
		for slot in 0..32 {
			let mut bdf = Bdf {
				bus,
				device: slot,
				func: 0
			};
			let vendor = pci_config_read::<WORD>(bdf, 0x00).unwrap();
			if vendor == 0xFFFF {
				continue;
			}

			handle_function(bdf, vendor);

			let header_type = pci_config_read::<WORD>(bdf, 0x0E).unwrap();
			let multifunction = (header_type & 0x80) != 0;

			if multifunction {
				for func in 1..8 {
					bdf.func = func;
					let vendor = pci_config_read::<WORD>(bdf, 0x00).unwrap();
					if vendor != 0xFFFF {
						handle_function(bdf, vendor);
					}
				}
			}
		}
	}

	serial_println!("[PCI] PCI device discovery complete");
}

fn handle_function(bdf: Bdf, vendor: u16) {
	let device = pci_config_read::<WORD>(bdf, 0x02).unwrap();

	let class_reg = pci_config_read::<WORD>(bdf, 0x0A).unwrap();
	let class = (class_reg >> 8) as u8;
	let subclass = (class_reg & 0xFF) as u8;
	let info = DriverInfo {
		vendor: Some(vendor),
		device: Some(device),
		class: Some(class),
		subclass: Some(subclass),
		probe: None
	};

	let dev = PciDevice::new_raw(bdf, info, None, None, None, None);

	let idx = add_pci_device(dev);

	serial_println!(
		"PCI {:02x}:{:02x}.{} vendor={:04x} device={:04x} class={:02x}:{:02x}",
		bdf.bus,
		bdf.device,
		bdf.func,
		vendor,
		device,
		class,
		subclass
	);

	try_bind_device(idx);
}

/// Try binds a PCI device to a valid driver.
pub fn try_bind_device(idx: usize) {
	let driver_infos = {
		let dt = DRIVER_TABLE.lock();
		if dt.is_empty() {
			return;
		}
		dt.clone()
	};

	let mut devices = PCI_DEVICES.lock();
	if idx >= devices.len() {
		return;
	}

	let dev = &mut devices[idx];

	if dev.bound_driver.is_some() {
		return;
	}

	for (_i, info) in driver_infos.iter().enumerate() {
		if matches(info, dev) {
			if let Some(probe_fn) = info.probe {
				match probe_fn(dev) {
					Ok(instance_idx) => {
						dev.bound_driver = Some(instance_idx);
						serial_println!(
							"Bound device {:?} to driver instance {}",
							dev.bdf,
							instance_idx
						);
						return;
					}
					Err(e) => {
						serial_println!("[PCI] Probe failed: {}", e);
					}
				}
			}
		}
	}
}

/// Enables the specified `PciDevice` for use.
pub fn pci_enable_device(dev: &mut PciDevice) -> Result<(), &'static str> {
	let bar_offset = 0x10;
	let orig = pci_config_read::<DWORD>(dev.bdf, bar_offset).unwrap();

	pci_config_write::<DWORD>(dev.bdf, bar_offset, 0xFFFF_FFFF)?;
	let mask = pci_config_read::<DWORD>(dev.bdf, bar_offset).unwrap();

	pci_config_write::<DWORD>(dev.bdf, bar_offset, orig)?;

	if (mask & 1) == 1 {
		let size_mask = mask & !0x3u32;
		let size = (!size_mask).wrapping_add(1);

		if size == 0 {
			return Err("I/O Bar Size == 0");
		}

		let assigned_base = orig & !0x3u32;
		if assigned_base != 0 {
			IO_ALLOC.lock().reserve(assigned_base, size);
			dev.io_base = Some(assigned_base as usize);
			dev.io_size = Some(size as usize);
		} else {
			match IO_ALLOC.lock().alloc(size, size) {
				Some(base) => {
					if (base & (size - 1)) != 0 {
						IO_ALLOC.lock().free(base, size);
						return Err("allocator returned misaligned I/O base");
					}

					dev.io_base = Some(base as usize);
					dev.io_size = Some(size as usize);

					let to_write = (base & !0x3u32) | 0x1u32;
					pci_config_write::<DWORD>(dev.bdf, bar_offset, to_write)?;
				}
				None => return Err("Unable to allocate I/O ports")
			}
		}

		let mut cmd = pci_config_read::<WORD>(dev.bdf, 0x04).unwrap();
		cmd |= PCI_COMMAND_IO;
		cmd |= PCI_BUS_MASTER;
		pci_config_write::<WORD>(dev.bdf, 0x04, cmd)?;

		serial_println!(
			"[PCI] Device: {:?} enabled (IO base={:#x}, size={:#x})",
			dev.bdf,
			dev.io_base.unwrap(),
			dev.io_size.unwrap()
		);
		return Ok(())
	} else {
		todo!("implement mmio pci device enabling");
	}
}

/// Find the PCI index from the GSI number.
pub fn pci_find_index_from_gsi(gsi: usize) -> Option<usize> {
	let devs = PCI_DEVICES.lock();
	for (idx, dev) in devs.iter().enumerate() {
		if dev.interrupt_line() as usize == gsi {
			return Some(idx);
		}
	}
	None
}
