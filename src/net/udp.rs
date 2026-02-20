//!
//! udp.rs
//! 
//! UDP packet logic for the kernel.
//! 

use alloc::vec::Vec;

use crate::{
	lazy_static,
	serial_println,
	utils::{mutex::SpinMutex, net::calculate_checksum}
};

lazy_static! {
	static ref UDP_HANDLERS: SpinMutex<Vec<(u16, fn(&[u8]))>> = SpinMutex::new(Vec::new());
}

/// Process incoming UDP packets.
pub fn process_udp(pkt: *const u8, len: usize, ip_offset: usize, _src_ip: &[u8; 4]) {
	let udp_offset = 14 + ip_offset;

	if len < udp_offset + 8 {
		serial_println!("[UDP] Packet too short");
		return;
	}

	unsafe {
		let udp_start = pkt.add(udp_offset);

		let src_port = u16::from_be_bytes([*udp_start.add(0), *udp_start.add(1)]);
		let dst_port = u16::from_be_bytes([*udp_start.add(2), *udp_start.add(3)]);
		let udp_length = u16::from_be_bytes([*udp_start.add(4), *udp_start.add(5)]);

		serial_println!(
			"[UDP] src_port={}, dst_port={}, len={}",
			src_port,
			dst_port,
			udp_length
		);

		let payload_len = (udp_length as usize).saturating_sub(8);
		if payload_len > 0 && len >= udp_offset + 8 + payload_len {
			let payload_ptr = udp_start.add(8);
			let payload = core::slice::from_raw_parts(payload_ptr, payload_len);

			// Find handler for this port
			let handlers = UDP_HANDLERS.lock();
			let handler_opt = handlers
				.iter()
				.find(|(port, _)| *port == dst_port)
				.or_else(|| handlers.iter().find(|(port, _)| *port == src_port));
			if let Some((_, handler)) = handler_opt {
				handler(payload);
			} else {
				serial_println!("[UDP] No handler for port {}", dst_port);
			}
		}
	}
}

/// Registers a UDP handler for incoming packets.
pub fn register_handler(port: u16, handler: fn(&[u8])) {
	let mut handlers = UDP_HANDLERS.lock();
	handlers.push((port, handler));
	serial_println!("[UDP] Registered handler for port {}", port);
}

/// Sends a UDP packet to the destination IP
pub fn send_udp(
	dst_ip: [u8; 4],
	src_port: u16,
	dst_port: u16,
	payload: &[u8]
) -> Result<(), &'static str> {
	let dst_mac = match super::get_next_hop_mac(dst_ip) {
		Ok(mac) => mac,
		Err(_) => {
			// resolve next hop
			let next_hop = if super::is_local_ip(dst_ip) {
				dst_ip
			} else {
				super::GATEWAY_IP
			};

			serial_println!(
				"[UDP] Resolving next hop MAC for {}.{}.{}.{}",
				next_hop[0],
				next_hop[1],
				next_hop[2],
				next_hop[3]
			);
			super::arp::send_arp_request(next_hop)?;
			return Err("MAC not cached");
		}
	};

	let our_mac = super::get_our_mac().ok_or("No MAC")?;

	let total_len = 14 + 20 + 8 + payload.len();
	let mut packet = alloc::vec![0u8; total_len];

	// ethernet header - use next hop MAC (might be gateway)
	packet[0..6].copy_from_slice(&dst_mac);
	packet[6..12].copy_from_slice(&our_mac);
	packet[12..14].copy_from_slice(&super::ethernet::ETHERTYPE_IPV4.to_be_bytes());

	// IPv4 header - use actual destination IP
	packet[14] = 0x45;
	packet[15] = 0;
	let ip_total_len = (20 + 8 + payload.len()) as u16;
	packet[16..18].copy_from_slice(&ip_total_len.to_be_bytes());
	packet[18..20].copy_from_slice(&1u16.to_be_bytes());
	packet[20..22].copy_from_slice(&0u16.to_be_bytes());
	packet[22] = 64;
	packet[23] = super::ipv4::IP_PROTO_UDP;
	packet[26..30].copy_from_slice(&super::OUR_IP);
	packet[30..34].copy_from_slice(&dst_ip);

	let ip_checksum = calculate_checksum(&packet[14..34]);
	packet[24..26].copy_from_slice(&ip_checksum.to_be_bytes());

	// UDP header
	packet[34..36].copy_from_slice(&src_port.to_be_bytes());
	packet[36..38].copy_from_slice(&dst_port.to_be_bytes());
	let udp_len = (8 + payload.len()) as u16;
	packet[38..40].copy_from_slice(&udp_len.to_be_bytes());
	packet[40..42].copy_from_slice(&0u16.to_be_bytes());

	// Payload
	packet[42..].copy_from_slice(payload);

	super::send_packet(&packet)?;
	serial_println!(
		"[UDP] Sent to {}.{}.{}.{}:{} via MAC {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
		dst_ip[0],
		dst_ip[1],
		dst_ip[2],
		dst_ip[3],
		dst_port,
		dst_mac[0],
		dst_mac[1],
		dst_mac[2],
		dst_mac[3],
		dst_mac[4],
		dst_mac[5]
	);
	Ok(())
}
