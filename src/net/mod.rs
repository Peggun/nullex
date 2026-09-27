//!
//! net/mod.rs
//!
//! Network module declaration.

pub mod arp;
pub mod dns;
pub mod ethernet;
pub mod http;
pub mod https;
pub mod icmp;
pub mod ipv4;
pub mod socket;
pub mod tcp;
pub mod udp;
pub mod util;

use core::net::Ipv4Addr;

use crate::{
	drivers::virtio::net::{VIRTIO_NET_INSTANCE, VirtioNet},
	error::NullexError,
	lazy_static,
	serial_println,
	sync::mutex::SpinMutex
};

lazy_static! {
	pub static ref NET_MANAGER: SpinMutex<Option<NetworkManager>> = SpinMutex::new(None);
}

pub struct NetworkManager {
	pub iface: Interface,
	pub device: VirtioNet, // todo create like a empty trait marker for network types.
	pub sockets: SocketSet<'static>
}

impl NetworkManager {
	pub fn poll(&mut self, timestamp: Instant) {
		self.iface
			.poll(timestamp, &mut self.device, &mut self.sockets);
	}
}

/// Our IP
/// currently manually set based on QEMU config.
pub const OUR_IP: [u8; 4] = [10, 0, 2, 15];
/// IP address to the Gateway
// manually set based on QEMU config.
pub const GATEWAY_IP: [u8; 4] = [10, 0, 2, 2];
/// Subnet Mask
// usually always 255.255.255.0 unless in like corporate.
pub const SUBNET_MASK: [u8; 4] = [255, 255, 255, 0];

/// Main point of receiving and handling packets.
pub fn receive_packet(pkt: *const u8, len: usize) {
	if len < 14 {
		serial_println!("[NET] Packet too short: {} bytes", len);
		return;
	}

	serial_println!("packet");

	unsafe {
		// ethernet header parse
		let ethertype = u16::from_be_bytes([*pkt.add(12), *pkt.add(13)]);

		let src_mac = [
			*pkt.add(6),
			*pkt.add(7),
			*pkt.add(8),
			*pkt.add(9),
			*pkt.add(10),
			*pkt.add(11)
		];

		serial_println!(
			"[NET] Received packet: {} bytes, ethertype: 0x{:04X}",
			len,
			ethertype
		);

		match ethertype {
			0x0806 => {
				serial_println!("[NET] -> ARP packet");
				arp::process_arp(pkt, len, src_mac);
			}
			0x0800 => {
				serial_println!("[NET] -> IPv4 packet");
				ipv4::process_ipv4(pkt, len);
			}
			_ => {
				serial_println!("[NET] -> Unknown ethertype: 0x{:04X}", ethertype);
			}
		}
	}
}

fn send_packet(packet: &[u8]) -> Result<(), NullexError> {
	crate::drivers::virtio::net::transmit_packet(packet)
}

fn get_our_mac() -> Option<[u8; 6]> {
	let mut guard = NET_MANAGER.lock();
	let manager = guard
		.as_mut()
		.ok_or(NullexError::NetworkNotInitialized)
		.expect("network is not initialized.");
	Some(manager.device.config.mac)
}

fn is_local_ip(ip: [u8; 4]) -> bool {
	for i in 0..4 {
		if (ip[i] & SUBNET_MASK[i]) != (OUR_IP[i] & SUBNET_MASK[i]) {
			return false;
		}
	}
	true
}

fn get_next_hop_mac(dst_ip: [u8; 4]) -> Result<[u8; 6], NullexError> {
	let next_hop_ip = if is_local_ip(dst_ip) {
		dst_ip
	} else {
		serial_println!(
			"[NET] {} is not local, routing through gateway",
			format_ip(dst_ip)
		);
		GATEWAY_IP
	};

	// next hop
	let cache = arp::ARP_CACHE.lock();
	cache
		.iter()
		.find(|(ip, _)| *ip == next_hop_ip)
		.map(|(_, mac)| *mac)
		.ok_or(NullexError::MacNotCached)
}

fn format_ip(ip: [u8; 4]) -> alloc::string::String {
	use alloc::format;
	format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

/// Initialise the Internet handlers.
pub fn init() {
	let mut instance = VIRTIO_NET_INSTANCE.lock();
	let (mut device, _) = instance.take().expect("nget: VirtioNet not initialized.");

	let mac = device.config.mac;

	let config = Config::new(EthernetAddress(mac).into());
	let mut iface = Interface::new(config, &mut device, crate::rtc::rtc_instant());

	iface.update_ip_addrs(|addrs| {
		addrs
			.push(IpCidr::new(
				IpAddress::Ipv4(Ipv4Addr::from_octets(OUR_IP)),
				24
			))
			.unwrap();
	});
	iface
		.routes_mut()
		.add_default_ipv4_route(Ipv4Addr::from_octets(GATEWAY_IP))
		.unwrap();

	let sockets = SocketSet::new(vec![]);

	serial_println!(
		"[NGET] Interface ready: {}.{}.{}.{}",
		OUR_IP[0],
		OUR_IP[1],
		OUR_IP[2],
		OUR_IP[3]
	);

	let mut guard = NET_MANAGER.lock();
	if guard.is_none() {
		*guard = Some(NetworkManager {
			iface,
			device,
			sockets
		});
	} else {
		let manager = guard.as_mut().unwrap();
		manager.device = device;
		manager.iface = iface;
		manager.sockets = sockets;
	}

	dns::init();
}

pub use arp::{ARP_CACHE, send_arp_request};
pub use icmp::send_ping;
use smoltcp::{
	iface::{Config, Interface, SocketSet},
	time::Instant,
	wire::{EthernetAddress, IpAddress, IpCidr}
};

// ---------- SYSCALLS ---------- //
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IoVec {
	pub iov_base: *mut u8,
	pub iov_len: usize
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MsgType {
	Raw = 0,
	Vec = 1
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RawPayload {
	pub buf: *mut u8,
	pub len: usize
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct VecPayload {
	pub msg_iov: *const IoVec,
	pub msg_iovcnt: usize
}

#[repr(C)]
#[derive(Copy, Clone)]
pub union MsgPayload {
	pub raw: RawPayload,
	pub vec: VecPayload
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct MsgHdr {
	pub msg_type: MsgType,
	pub _padding: u32,
	pub payload: MsgPayload
}
