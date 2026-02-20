//!
//! net.rs
//! 
//! VirtIO Network Driver Specification based module for the kernel.
//! 

use alloc::vec::Vec;
use core::ptr::write_bytes;

use x86_64::{align_up, structures::idt::InterruptStackFrame};

use crate::{
	apic::send_eoi,
	common::ports::{inb, inw, outl, outw},
	drivers::virtio::{
		VIRTIO_IO_DEVICE_CFG,
		VIRTIO_IO_DEVICE_FEATURES,
		VIRTIO_IO_DEVICE_STATUS,
		VIRTIO_IO_DRIVER_FEATURES,
		VIRTIO_IO_ISR,
		VIRTIO_IO_QUEUE_ADDR,
		VIRTIO_IO_QUEUE_SELECT,
		VIRTIO_IO_QUEUE_SIZE,
		VirtIODeviceStatus,
		VirtQueue,
		VirtioDevice,
		VirtqueueAvailable,
		VirtqueueDescriptor,
		VirtqueueUsed,
		virtqueue_size
	},
	gsi::GSI_TABLE,
	io::{
		io_read,
		io_write,
		pci::{DriverInfo, PciDevice, VIRTIO_PCI_VENDOR_ID, pci_enable_device, register_driver}
	},
	lazy_static,
	memory::{DmaBuffer, dma_alloc},
	serial_println,
	utils::{
		endian::{Le16, Le32},
		mutex::SpinMutex,
		types::{BYTE, QWORD}
	}
};

lazy_static! {
	/// Static reference to the VirtioNet Device.
	pub static ref VIRTIO_NET_DEVICE: SpinMutex<Option<VirtioNetDevice>> = SpinMutex::new(None);
	/// Static reference to the RX Queue
	pub static ref RX_QUEUE: SpinMutex<VirtQueue> = SpinMutex::new(VirtQueue::empty());
	/// Static reference to the TX Queue
	pub static ref TX_QUEUE: SpinMutex<VirtQueue> = SpinMutex::new(VirtQueue::empty());
	/// Static reference to the RX Buffers
	pub static ref RX_BUFFERS: SpinMutex<Vec<Option<DmaBuffer>>> =
		SpinMutex::new(Vec::with_capacity(256));
	/// Static reference to the VirtIO net instance.
	pub static ref VIRTIO_NET_INSTANCE: SpinMutex<Option<(VirtioNet, usize)>> =
		SpinMutex::new(None);
	/// Static reference to the TX Inflight.
	pub static ref TX_INFLIGHT: SpinMutex<Vec<Option<DmaBuffer>>> = SpinMutex::new(Vec::new());
}

/// Structure to store device-specific data for interrupt handler
pub struct VirtioNetDevice {
	/// Base IO address
	pub io_base: u16,
	/// Global System Interrupt number
	pub gsi: u8,
	/// Interrupt Vector
	pub vector: u8
}

// https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html#x1-2340001
const VIRTIO_DEVICE_ID: u8 = 1;
const VIRTIO_NET_IDT_VECTOR: u8 = 34;

const NET_DRIVER_SUPPORTED_FEATURES: u64 = VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS;

const VIRTIO_NET_RX_BUFFERS: u64 = 256;

// other virtqueues like (2n+1) arent implemented

// Feature bits
/// Device handles packets with partial checksum.
const VIRTIO_NET_F_CSUM: u64 = 1 << 0;
/// Driver handles packets with partial checksum.
const VIRTIO_NET_F_GUEST_CSUM: u64 = 1 << 1;
/// Control channel offloads reconfiguration support.
const VIRTIO_NET_F_GUEST_OFFLOADS: u64 = 1 << 2;
/// Device maximum MTU (Maximum Transmission Unit) reporting is supported.
const VIRTIO_NET_F_MTU: u64 = 1 << 3;
// 4 not in use
/// Device has given MAC address.
const VIRTIO_NET_F_MAC: u64 = 1 << 5;
// 6 not in use
/// Driver can receive TSOv4.
/// Requires `VIRTIO_NET_F_GUEST_CSUM`
const VIRTIO_NET_F_GUEST_TSO4: u64 = 1 << 7;
/// Driver can receive TSOv6.
/// Requires `VIRTIO_NET_F_GUEST_CSUM`
const VIRTIO_NET_F_GUEST_TSO6: u64 = 1 << 8;
/// Driver can receive TSO with ECN.
/// Requires `VIRTIO_NET_F_GUEST_TSO4` or `VIRTIO_NET_F_GUEST_TSO6`.
pub const VIRTIO_NET_F_GUEST_ECN: u64 = 1 << 9;
/// Driver can receive UFO.
/// Requires `VIRTIO_NET_F_GUEST_CSUM`.
const VIRTIO_NET_F_GUEST_UFO: u64 = 1 << 10;
/// Device can receive TSOv4.
/// Requires `VIRTIO_NET_F_CSUM`.
const VIRTIO_NET_F_HOST_TSO4: u64 = 1 << 11;
/// Device can receive TSOv6.
/// Requires `VIRTIO_NET_F_CSUM`.
const VIRTIO_NET_F_HOST_TSO6: u64 = 1 << 12;
/// Device can receive TSO with ECN.
/// Requires `VIRTIO_NET_F_HOST_TSO4` or `VIRTIO_NET_F_HOST_TSO6`.
const VIRTIO_NET_F_HOST_ECN: u64 = 1 << 13;
/// Device can receive UFO.
/// Requires `VIRTIO_NET_F_CSUM`.
const VIRTIO_NET_F_HOST_UFO: u64 = 1 << 14;
/// Driver can merge receive buffers.
const VIRTIO_NET_F_MRG_RXBUF: u64 = 1 << 15;
/// Configuration status field is available.
const VIRTIO_NET_F_STATUS: u64 = 1 << 16;
/// Control channel is available.
const VIRTIO_NET_F_CTRL_VQ: u64 = 1 << 17;
/// Control channel RX mode support.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
const VIRTIO_NET_F_CTRL_RX: u64 = 1 << 18;
/// Control channel VLAN filtering.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
const VIRTIO_NET_F_CTRL_VLAN: u64 = 1 << 19;
/// Control channel RX extra mode support.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
const VIRTIO_NET_F_CTRL_RX_EXTRA: u64 = 1 << 20;
/// Driver can send gratuitous packets.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
const VIRTIO_NET_F_GUEST_ANNOUNCE: u64 = 1 << 21;
/// Driver supports multiqueue with automatic receive steering.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
const VIRTIO_NET_F_MQ: u64 = 1 << 22;
/// Set MAC address through control channel.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
const VIRTIO_NET_F_CTRL_MAC_ADDR: u64 = 1 << 23;
// 24-32 not in use
const VIRTIO_F_VERSION_1: u64 = 1 << 32;
// 33-50 not in use
/// Device supports inner header hash for encapsulated packets.
/// Requires `VIRTIO_NET_F_CTRL_VQ` along with
/// `VIRTIO_NET_F_RSS` or `VIRTIO_NET_F_HASH_REPORT`.
const VIRTIO_NET_F_HASH_TUNNEL: u64 = 1 << 51;
/// Device supports `VirtQueues` notification coalescing.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
const VIRTIO_NET_F_VQ_NOTF_COAL: u64 = 1 << 52;
/// Device supports notification coalescing.
const VIRTIO_NET_F_NOTF_COAL: u64 = 1 << 53;
/// Driver can receive USOv4 packets.
const VIRTIO_NET_F_GUEST_USO4: u64 = 1 << 54;
/// Driver can receive USOv6 packets.   
const VIRTIO_NET_F_GUEST_USO6: u64 = 1 << 55;
/// Device can receive USO packets.
/// Requires `VIRTIO_NET_F_CSUM`.
const VIRTIO_NET_F_HOST_USO: u64 = 1 << 56;
/// Device can report per-packet hash value and a type of calculated hash.
const VIRTIO_NET_F_HASH_REPORT: u64 = 1 << 57;
// 58 not in use
/// Driver can provide the exact `hdr_len` value. Device benefits from knowing
/// the exact header length.
const VIRTIO_NET_F_GUEST_HDRLEN: u64 = 1 << 59;
/// Device supports RSS (receive-side scaling) with Toeplitz hash calculation
/// and configurable hash parameters for receive steering.
/// Requires `VIRTIO_NET_F_CTRL_VQ`.
const VIRTIO_NET_F_RSS: u64 = 1 << 60;
/// Device can process duplicated ACKs and
/// report number of coalesced segments and duplicated ACKS.
/// Requires `VIRTIO_NET_F_HOST_TSO4` or `VIRTIO_NET_F_HOST_TSO6`.
const VIRTIO_NET_F_RSC_EXT: u64 = 1 << 61;
/// Device may act as a standby for a primary device with the same MAC address.
const VIRTIO_NET_F_STANDBY: u64 = 1 << 62;
/// Device reports speed and duplex.
const VIRTIO_NET_F_SPEED_DUPLEX: u64 = 1 << 63;

// header values
// flags
const VIRTIO_NET_HDR_F_NEEDS_CSUM: u64 = 1;
const VIRTIO_NET_HDR_F_DATA_VALID: u64 = 2;
const VIRTIO_NET_HDR_F_RSC_INFO: u64 = 4;
// gso types
const VIRTIO_NET_HDR_GSO_NONE: u64 = 0;
const VIRTIO_NET_HDR_GSO_TCPV4: u64 = 1;
const VIRTIO_NET_HDR_GSO_UDP: u64 = 3;
const VIRTIO_NET_HDR_GSO_TCPV6: u64 = 4;
const VIRTIO_NET_HDR_GSO_UDP_L4: u64 = 5;
const VIRTIO_NET_HDR_GSO_ECN: u64 = 0x80;

//#[repr(C)]
#[derive(Debug, Default)]
/// Structure representing the VirtioNet Configuration.
pub struct VirtioNetConfig {
	/// MAC address of the device.
	pub mac: [u8; 6],
	/// The status of the device.
	pub status: Option<Le16>,
	/// The maximum `VirtQueue` pairs of the device.
	pub max_virtqueue_pairs: Option<Le16>,
	/// The maximum transmission unit of the device.
	pub mtu: Option<Le16>,
	/// The speed of the device.
	pub speed: Option<Le32>,
	/// If the device and send & receive data simultaneously
	pub duplex: Option<u8>,
	/// The RSS maximum key size of the device.
	pub rss_max_key_size: Option<u8>,
	/// The RSS maximum indirection table length of the device.
	pub rss_max_indirection_table_length: Option<Le16>,
	/// All supported hash types of the device.
	pub supported_hash_types: Option<Le32>,
	/// All supported tunnel types of the device.
	pub supported_tunnel_types: Option<Le32>
}

impl VirtioNetConfig {
	/// Create a new `VirtioNetConfig` with a specified MAC address.
	pub fn new(mac: [u8; 6]) -> VirtioNetConfig {
		Self {
			mac,
			status: None,
			max_virtqueue_pairs: None,
			mtu: None,
			speed: None,
			duplex: None,
			rss_max_key_size: None,
			rss_max_indirection_table_length: None,
			supported_hash_types: None,
			supported_tunnel_types: None
		}
	}
}

#[repr(C)]
#[derive(Default)]
/// Structure representing a VirtioNet Header.
pub struct VirtioNetHeader {
	flags: u8,
	gso_type: u8,
	hdr_len: Le16,
	gso_size: Le16,
	csum_start: Le16,
	csum_offset: Le16
	// pub hash_value: Option<Le16>,
	// pub hash_report: Option<Le16>,
	// pub padding_reserved: Option<Le16>
}

// sanity
const _: () = assert!(core::mem::size_of::<VirtioNetHeader>() == 10);

/// Structure representing the Virtio Network device.
pub struct VirtioNet {
	/// The base IO address of the device.
	pub io_base: usize,
	/// The header for the device.
	pub header: VirtioNetHeader,
	/// The configuration for the device.
	pub config: VirtioNetConfig,
	/// All features that are currently active on the device.
	pub negotiated_features: u64,
	/// The receiving queue for the device.
	pub rx_queue: Option<VirtQueue>,
	/// The transmit queue for the device.
	pub tx_queue: Option<VirtQueue>,
	/// The control queue for the device.
	pub ctrl_queue: Option<VirtQueue>
}

impl VirtioNet {
	/// Creates a new `VirtioNet` device. 
	pub fn new( 
		io_base: usize,
		header: VirtioNetHeader,
		config: VirtioNetConfig,
		nf: u64,
		rx: Option<VirtQueue>,
		tx: Option<VirtQueue>,
		ctrl: Option<VirtQueue>
	) -> VirtioNet {
		Self {
			io_base,
			header,
			config,
			negotiated_features: nf,
			rx_queue: rx,
			tx_queue: tx,
			ctrl_queue: ctrl
		}
	}
}

impl VirtioDevice for VirtioNet {
	fn alloc_virtqueue(&mut self, qidx: u16) -> Result<VirtQueue, &'static str> {
		unsafe {
			outw(
				(self.io_base + VIRTIO_IO_QUEUE_SELECT).try_into().unwrap(),
				qidx
			);
			let size = inw((self.io_base + VIRTIO_IO_QUEUE_SIZE).try_into().unwrap());
			if size == 0 {
				return Err("queue not available");
			}

			let layout_size = virtqueue_size(size as usize);
			let (virt_addr, phys_addr) = dma_alloc(layout_size).ok_or("dma_alloc failed")?;
			write_bytes(virt_addr.as_mut_ptr::<u8>(), 0, layout_size);

			outl(
				(self.io_base + VIRTIO_IO_QUEUE_ADDR).try_into().unwrap(),
				(phys_addr.as_u64() >> 12) as u32
			);

			let mut vq = VirtQueue {
				size,
				desc: virt_addr.as_mut_ptr::<VirtqueueDescriptor>(),
				avail: (virt_addr
					.as_mut_ptr::<u8>()
					.add(core::mem::size_of::<VirtqueueDescriptor>() * size as usize))
					as *mut VirtqueueAvailable,
				used: (virt_addr.as_mut_ptr::<u8>().add(
					align_up(
						(core::mem::size_of::<VirtqueueDescriptor>() * size as usize
							+ core::mem::size_of::<VirtqueueAvailable>()
							+ size as usize * 2)
							.try_into()
							.unwrap(),
						4096
					)
					.try_into()
					.unwrap()
				)) as *mut VirtqueueUsed,
				free_head: 0,
				last_used: 0,
				num_free: size,
				phys_addr,
				virt_addr,
				queue_index: qidx,
				io_base: self.io_base as u16
			};
			vq.init_free_list();
			Ok(vq)
		}
	}

	fn device_features(&mut self) -> u64 {
		if self.negotiated_features == 0 {
			io_read::<QWORD>(self.io_base, VIRTIO_IO_DEVICE_FEATURES).unwrap()
		} else {
			self.negotiated_features
		}
	}

	fn set_driver_features(&mut self, features: u64) {
		self.negotiated_features = features;
		io_write::<QWORD>(self.io_base, VIRTIO_IO_DRIVER_FEATURES, features).unwrap();
	}

	fn driver_status(&mut self) -> u16 {
		if let Some(cur_status) = self.config.status {
			cur_status
		} else {
			let status = io_read::<BYTE>(self.io_base, VIRTIO_IO_DEVICE_STATUS).unwrap();
			self.set_driver_status(status);
			status as u16
		}
	}

	fn set_driver_status(&mut self, status: u8) {
		let new_status: u16 = match self.config.status {
			Some(current) => {
				if status == VirtIODeviceStatus::FAILED.bits() {
					status as u16
				} else {
					current | (status as u16)
				}
			}
			None => status as u16
		};
		self.config.status = Some(new_status);
		io_write::<BYTE>(self.io_base, VIRTIO_IO_DEVICE_STATUS, new_status as u8).unwrap();
	}

	fn has_status(&mut self, status: u8) -> bool {
		(self.driver_status() & (status as u16)) != 0
	}

	fn supported_features(&mut self) -> u64 {
		self.negotiated_features
	}

	fn init(&mut self) -> Result<(), &'static str> {
		let supported = self.supported_features();
		let want = supported & NET_DRIVER_SUPPORTED_FEATURES;
		self.set_driver_features(want);

		let mut rx_vq = self.alloc_virtqueue(0)?;
		let rx_queue_size = rx_vq.size as usize;

		serial_println!("[VIRTIO-NET] RX queue size: {}", rx_queue_size);

		{
			let mut rx_buffers = RX_BUFFERS.lock();
			rx_buffers.clear();
			rx_buffers.resize_with(rx_queue_size, || None);
		}

		for _ in 0..rx_queue_size {
			let buf_size = 1500 + core::mem::size_of::<VirtioNetHeader>();
			let (virt_addr, phys_addr) = dma_alloc(buf_size).expect("DMA alloc failed");
			unsafe { write_bytes(virt_addr.as_mut_ptr::<u8>(), 0, buf_size) }

			let desc_id = rx_vq.add_descriptor(phys_addr, buf_size as u32, true)?;
			{
				let mut rx_buffers = RX_BUFFERS.lock();
				if desc_id as usize >= rx_buffers.len() {
					rx_buffers.resize_with(desc_id as usize + 1, || None);
				}
				rx_buffers[desc_id as usize] = Some(DmaBuffer {
					phys: phys_addr,
					virt: virt_addr,
					len: buf_size
				});
			}

			rx_vq.push_avail(desc_id as u16);
		}

		serial_println!(
			"[VIRTIO-NET] RX queue prepared with {} buffers (kick deferred)",
			rx_queue_size
		);
		*RX_QUEUE.lock() = rx_vq;

		let tx_vq = self.alloc_virtqueue(1)?;
		*TX_QUEUE.lock() = tx_vq;

		serial_println!("[VIRTIO-NET] Device initialized (queues ready, DRIVER_OK not set yet)");
		Ok(())
	}
}

fn handle_rx_packet(desc_id: u16, len: u32) {
	let hdr_len = core::mem::size_of::<VirtioNetHeader>();

	let pkt_ptr = {
		let rx_buffers = RX_BUFFERS.lock();
		let buf_opt = rx_buffers.get(desc_id as usize).and_then(|o| o.as_ref());
		if buf_opt.is_none() {
			serial_println!("[VIRTIO-NET] ERROR: No buffer at desc_id {}", desc_id);
			return;
		}
		let buf = buf_opt.unwrap();
		unsafe { buf.virt.as_ptr::<u8>().add(hdr_len) }
	};

	let pkt_len = (len as usize).saturating_sub(hdr_len);
	serial_println!(
		"[VIRTIO-NET] RX packet ({} bytes) desc_id={}",
		pkt_len,
		desc_id
	);

	unsafe {
		if pkt_len >= 14 {
			let ethertype = u16::from_be_bytes([
				core::ptr::read(pkt_ptr.add(12)),
				core::ptr::read(pkt_ptr.add(13))
			]);
			serial_println!("[VIRTIO-NET] Ethernet ethertype=0x{:04x}", ethertype);
		}
	}

	// call network stack
	crate::net::receive_packet(pkt_ptr, pkt_len);

	{
		let mut rx_queue = RX_QUEUE.lock();
		rx_queue.push_avail(desc_id);
		rx_queue.kick();
	}
}

fn _rx_replenish_one(desc_id: u16, _old_buf: DmaBuffer) {
	serial_println!("[VIRTIO-NET] Replenish: requeue desc_id={}", desc_id);
	let mut rx_queue = RX_QUEUE.lock();
	rx_queue.push_avail(desc_id);
	rx_queue.kick();
}

/// Transmit a packet to the transport queue (TX)
pub fn transmit_packet(packet: &[u8]) -> Result<(), &'static str> {
	serial_println!("[VIRTIO-NET] TX packet ({} bytes)", packet.len());
	serial_println!("[VIRTIO-NET] Packet contents (Ethernet header):");
	serial_println!(
		"  Dst MAC: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
		packet[0],
		packet[1],
		packet[2],
		packet[3],
		packet[4],
		packet[5]
	);
	serial_println!(
		"  Src MAC: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
		packet[6],
		packet[7],
		packet[8],
		packet[9],
		packet[10],
		packet[11]
	);
	serial_println!("  EtherType: 0x{:02X}{:02X}", packet[12], packet[13]);

	const HEADER_SIZE: usize = core::mem::size_of::<VirtioNetHeader>();
	let total_size = HEADER_SIZE + packet.len();
	let (virt_addr, phys_addr) = dma_alloc(total_size).ok_or("TX buffer alloc failed")?;

	unsafe {
		let header = VirtioNetHeader::default();
		let header_ptr = virt_addr.as_mut_ptr::<VirtioNetHeader>();
		core::ptr::write(header_ptr, header);

		let packet_ptr = virt_addr.as_mut_ptr::<u8>().add(HEADER_SIZE);
		core::ptr::copy_nonoverlapping(packet.as_ptr(), packet_ptr, packet.len());
	}

	let tx_buffer = DmaBuffer {
		phys: phys_addr,
		virt: virt_addr,
		len: total_size
	};

	let mut tx_inflight = TX_INFLIGHT.lock();
	let mut tx_queue = TX_QUEUE.lock();

	let desc_id = tx_queue.add_descriptor(phys_addr, total_size as u32, false)?;
	tx_queue.push_avail(desc_id);
	tx_queue.kick();

	while tx_inflight.len() <= desc_id as usize {
		tx_inflight.push(None);
	}
	tx_inflight[desc_id as usize] = Some(tx_buffer);

	serial_println!(
		"[VIRTIO-NET] TX queued (desc_id={}, phys={:#x}, len={})",
		desc_id,
		phys_addr.as_u64(),
		total_size
	);
	Ok(())
}

fn virtio_net_finalize() -> Result<(), &'static str> {
	serial_println!("[VIRTIO-NET] Finalizing device (setting DRIVER_OK)");

	let mut instance = VIRTIO_NET_INSTANCE.lock();
	if let Some((ref mut virtio_net, io_base)) = *instance {
		virtio_net.set_driver_status(VirtIODeviceStatus::DRIVER_OK.bits());
		serial_println!("[VIRTIO-NET] Device finalized at io_base={:#x}", io_base);
		Ok(())
	} else {
		Err("No VirtIO network instance")
	}
}

/// Initialize the Virtio Net driver.
pub fn virtio_net_driver_init() {
	serial_println!("[VIRTIO-NET] Registering driver");
	register_driver(DriverInfo {
		vendor: Some(VIRTIO_PCI_VENDOR_ID),
		device: None,
		class: None,
		subclass: None,
		probe: Some(virtio_net_probe)
	});
}

/// Probe the virtio net device.
pub fn virtio_net_probe(dev: &mut PciDevice) -> Result<usize, &'static str> {
	serial_println!("[VIRTIO-NET] Probing device {:?}", dev.bdf);

	pci_enable_device(dev)?;
	let io_base = dev.io_base.ok_or("no io base")?;

	let mut virtio_net = VirtioNet::new(
		io_base,
		VirtioNetHeader::default(),
		VirtioNetConfig::default(),
		0,
		None,
		None,
		None
	);

	virtio_net.set_driver_status(0);
	virtio_net.set_driver_status(
		VirtIODeviceStatus::ACKNOWLEDGE
			.union(VirtIODeviceStatus::DRIVER)
			.bits()
	);

	let dev_features = virtio_net.device_features();
	let driv_ok_features = dev_features & NET_DRIVER_SUPPORTED_FEATURES;

	virtio_net.set_driver_features(driv_ok_features);
	virtio_net.set_driver_status(VirtIODeviceStatus::FEATURES_OK.bits());

	if !virtio_net.has_status(VirtIODeviceStatus::FEATURES_OK.bits()) {
		virtio_net.set_driver_status(VirtIODeviceStatus::FAILED.bits());
		return Err("device rejected features");
	}

	let mac = {
		let mut value = [0u8; 6];
		for i in 0..6 {
			value[i] = unsafe { inb((io_base + VIRTIO_IO_DEVICE_CFG + i) as u16) };
		}
		value
	};

	serial_println!(
		"[VIRTIO-NET] MAC: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
		mac[0],
		mac[1],
		mac[2],
		mac[3],
		mac[4],
		mac[5]
	);
	virtio_net.config.mac = mac;

	virtio_net.init()?;
	virtio_net.set_driver_status(VirtIODeviceStatus::DRIVER_OK.bits());

	serial_println!("[VIRTIO-NET] DRIVER_OK status set");

	// Verify DRIVER_OK is actually set
	let status = unsafe { inb((io_base + VIRTIO_IO_DEVICE_STATUS) as u16) };
	serial_println!("[VIRTIO-NET] Device status register: {:#x}", status);
	if (status & VirtIODeviceStatus::DRIVER_OK.bits()) == 0 {
		return Err("DRIVER_OK not set!");
	}

	{
		let rx_queue = RX_QUEUE.lock();
		rx_queue.kick();
		serial_println!("[VIRTIO-NET] RX queue kicked AFTER DRIVER_OK");
	}

	*VIRTIO_NET_INSTANCE.lock() = Some((virtio_net, io_base));

	let gsi = dev.interrupt_line() as usize;
	serial_println!("[VIRTIO-NET] Device uses GSI {}", gsi);

	*VIRTIO_NET_DEVICE.lock() = Some(VirtioNetDevice {
		io_base: io_base as u16,
		gsi: gsi as u8,
		vector: VIRTIO_NET_IDT_VECTOR
	});

	{
		let mut gt = GSI_TABLE.lock();
		if gsi < 256 {
			gt[gsi].device_ptr = Some(dev as *const _ as usize);
			gt[gsi].handler = Some(virtio_net_interrupt_handler);
			gt[gsi].vector = Some(VIRTIO_NET_IDT_VECTOR); // Store the vector
			serial_println!("[VIRTIO-NET] Registered handler for GSI {}", gsi);
		}
	}

	unsafe {
		let mut ioapic = crate::ioapic::IOAPIC.lock();

		let mut entry = crate::ioapic::RedirectionTableEntry::default();
		entry.set_vector(VIRTIO_NET_IDT_VECTOR); // Use 34, NOT 32+gsi!
		entry.set_mode(crate::ioapic::IrqMode::Fixed);
		entry.set_dest(0); // BSP APIC ID
		entry.set_flags(crate::ioapic::IrqFlags::empty()); // UNMASKED!

		ioapic.set_table_entry(gsi as u8, entry);

		serial_println!(
			"[VIRTIO-NET] IOAPIC RTE configured for GSI {} -> vector {}",
			gsi,
			VIRTIO_NET_IDT_VECTOR
		);

		// verify it was set correctly
		let verify = ioapic.table_entry(gsi as u8);
		serial_println!(
			"[VIRTIO-NET] Verified RTE: vector={}, masked={}",
			verify.vector(),
			verify.mask()
		);
	}

	// Dump the GSI to verify
	crate::ioapic::dump_gsi(gsi as u8);

	serial_println!("[VIRTIO-NET] Probe complete - interrupts should now work!");
	Ok(0)
}

/// VirtioNet Interrupt Handler.
pub extern "x86-interrupt" fn virtio_net_interrupt_handler(_stack_frame: InterruptStackFrame) {
	serial_println!("[VIRTIO-NET] Interrupt!");

	let io_base = {
		let dev = VIRTIO_NET_DEVICE.lock();
		match dev.as_ref() {
			Some(d) => d.io_base as usize,
			None => {
				unsafe {
					send_eoi();
				}
				return;
			}
		}
	};

	let isr = unsafe { inb(io_base as u16 + VIRTIO_IO_ISR as u16) };
	serial_println!("[VIRTIO-NET] ISR={:#x}", isr);

	if (isr & 0x1) != 0 {
		serial_println!("[VIRTIO-NET] Queue interrupt");
		rx_poll();
		tx_poll();
	}

	unsafe {
		send_eoi();
	}
}

fn tx_poll() {
	//serial_println!("[VIRTIO-NET] Polling TX queue");

	let completions = {
		let mut tx_queue = TX_QUEUE.lock();
		let mut completions = Vec::new();
		while let Some((desc_id, _len)) = tx_queue.pop_used() {
			completions.push(desc_id);
		}
		completions
	};

	if !completions.is_empty() {
		serial_println!(
			"[VIRTIO-NET] Processing {} TX completions",
			completions.len()
		);
		let mut tx_inflight = TX_INFLIGHT.lock();
		for desc_id in completions.iter() {
			serial_println!("[VIRTIO-NET] TX completed desc_id={}", desc_id);
			if (*desc_id as usize) < tx_inflight.len() {
				tx_inflight[*desc_id as usize] = None;
			}
		}
	}
}

/// Poll the receive queue. (RX)
pub fn rx_poll() {
	//serial_println!("[VIRTIO-NET] Polling RX queue");

	let packets = {
		let mut rx_queue = RX_QUEUE.lock();
		let mut packets = Vec::new();
		while let Some((desc_id, len)) = rx_queue.pop_used() {
			packets.push((desc_id, len));
		}
		packets
	};

	for (desc_id, len) in packets.iter() {
		serial_println!("[VIRTIO-NET] Processing desc_id={}, len={}", desc_id, len);
		handle_rx_packet(*desc_id, *len);
	}
	//serial_println!("[VIRTIO-NET] Processed {} packets", packets.len());
}
