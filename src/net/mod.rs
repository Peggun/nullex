//!
//! net/mod.rs
//! 
//! Network module declaration.
//! 

pub mod arp;
pub mod dns;
pub mod ethernet;
pub mod icmp;
pub mod ipv4;
pub mod udp;

use crate::{drivers::virtio::net::VIRTIO_NET_INSTANCE, serial_println};

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

fn send_packet(packet: &[u8]) -> Result<(), &'static str> {
	crate::drivers::virtio::net::transmit_packet(packet)
}

fn get_our_mac() -> Option<[u8; 6]> {
	VIRTIO_NET_INSTANCE
		.lock()
		.as_ref()
		.map(|(net, _)| net.config.mac)
}

fn is_local_ip(ip: [u8; 4]) -> bool {
	for i in 0..4 {
		if (ip[i] & SUBNET_MASK[i]) != (OUR_IP[i] & SUBNET_MASK[i]) {
			return false;
		}
	}
	true
}

fn get_next_hop_mac(dst_ip: [u8; 4]) -> Result<[u8; 6], &'static str> {
	let next_hop_ip = if is_local_ip(dst_ip) {
		dst_ip
	} else {
		serial_println!(
			"[NET] {} is not local, routing through gateway",
			format_ip(dst_ip)
		);
		GATEWAY_IP
	};

	let cache = arp::ARP_CACHE.lock();
	cache
		.iter()
		.find(|(ip, _)| *ip == next_hop_ip)
		.map(|(_, mac)| *mac)
		.ok_or("Next hop MAC not cached")
}

fn format_ip(ip: [u8; 4]) -> alloc::string::String {
	use alloc::format;
	format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

/// Initialise the Internet handlers. (DNS currently)
pub fn init() {
	dns::init();
}

// Re-exports
pub use arp::{ARP_CACHE, send_arp_request};
pub use icmp::send_ping;