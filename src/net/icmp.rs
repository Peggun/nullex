//!
//! icmp.rs
//! 
//! ICMP packet handling logic for the kernel.
//! 

use crate::{serial_println, utils::net::calculate_checksum};

const ICMP_ECHO_REPLY: u8 = 0;
const ICMP_ECHO_REQUEST: u8 = 8;

/// Process incoming ICMP packets.
pub fn process_icmp(pkt: *const u8, len: usize, ip_header_len: usize, src_ip: &[u8; 4]) {
	let icmp_offset = 14 + ip_header_len;

	if len < icmp_offset + 8 {
		serial_println!("[ICMP] Packet too short");
		return;
	}

	unsafe {
		let icmp_start = pkt.add(icmp_offset);

		let icmp_type = *icmp_start;
		let id = u16::from_be_bytes([*icmp_start.add(4), *icmp_start.add(5)]);
		let sequence = u16::from_be_bytes([*icmp_start.add(6), *icmp_start.add(7)]);

		serial_println!("[ICMP] type={}, id={}, seq={}", icmp_type, id, sequence);

		match icmp_type {
			ICMP_ECHO_REQUEST => {
				serial_println!("[ICMP] Echo request, sending reply");
				send_icmp_reply(pkt, len, src_ip, id, sequence);
			}
			ICMP_ECHO_REPLY => {
				serial_println!(
					"[ICMP] Echo reply from {}.{}.{}.{}: seq={}",
					src_ip[0],
					src_ip[1],
					src_ip[2],
					src_ip[3],
					sequence
				);
			}
			_ => {
				serial_println!("[ICMP] Unknown type: {}", icmp_type);
			}
		}
	}
}

fn send_icmp_reply(
	original_pkt: *const u8,
	original_len: usize,
	dst_ip: &[u8; 4],
	id: u16,
	sequence: u16
) {
	let our_mac = match super::get_our_mac() {
		Some(mac) => mac,
		None => {
			serial_println!("[ICMP] No MAC");
			return;
		}
	};

	let dst_mac = {
		let cache = super::arp::ARP_CACHE.lock();
		cache
			.iter()
			.find(|(ip, _)| ip == dst_ip)
			.map(|(_, mac)| *mac)
	};

	let dst_mac = match dst_mac {
		Some(mac) => mac,
		None => {
			serial_println!("[ICMP] MAC not in cache");
			return;
		}
	};

	let ip_header_len = unsafe {
		let version_ihl = *original_pkt.add(14);
		((version_ihl & 0x0F) as usize) * 4
	};
	let icmp_offset = 14 + ip_header_len;
	let payload_len = original_len - icmp_offset - 8;

	let total_len = 14 + 20 + 8 + payload_len;
	let mut packet = alloc::vec![0u8; total_len];

	// ethernet header
	packet[0..6].copy_from_slice(&dst_mac);
	packet[6..12].copy_from_slice(&our_mac);
	packet[12..14].copy_from_slice(&super::ethernet::ETHERTYPE_IPV4.to_be_bytes());

	// IPv4 header
	packet[14] = 0x45;
	packet[15] = 0;
	let ip_total_len = (20 + 8 + payload_len) as u16;
	packet[16..18].copy_from_slice(&ip_total_len.to_be_bytes());
	packet[18..20].copy_from_slice(&1u16.to_be_bytes());
	packet[20..22].copy_from_slice(&0u16.to_be_bytes());
	packet[22] = 64;
	packet[23] = super::ipv4::IP_PROTO_ICMP;
	packet[26..30].copy_from_slice(&super::OUR_IP);
	packet[30..34].copy_from_slice(dst_ip);

	let ip_checksum = calculate_checksum(&packet[14..34]);
	packet[24..26].copy_from_slice(&ip_checksum.to_be_bytes());

	// ICMP header
	packet[34] = ICMP_ECHO_REPLY;
	packet[35] = 0;
	packet[38..40].copy_from_slice(&id.to_be_bytes());
	packet[40..42].copy_from_slice(&sequence.to_be_bytes());

	if payload_len > 0 {
		unsafe {
			let src_payload = original_pkt.add(icmp_offset + 8);
			packet[42..42 + payload_len]
				.copy_from_slice(core::slice::from_raw_parts(src_payload, payload_len));
		}
	}

	let icmp_checksum = calculate_checksum(&packet[34..]);
	packet[36..38].copy_from_slice(&icmp_checksum.to_be_bytes());

	if let Err(e) = super::send_packet(&packet) {
		serial_println!("[ICMP] Failed to send: {}", e);
	} else {
		serial_println!("[ICMP] Reply sent");
	}
}

/// Sends a PING packet to a destination. 
pub fn send_ping(dst_ip: [u8; 4], sequence: u16) -> Result<(), &'static str> {
	let dst_mac = match super::get_next_hop_mac(dst_ip) {
		Ok(mac) => mac,
		Err(_) => {
			let next_hop = if super::is_local_ip(dst_ip) {
				dst_ip
			} else {
				super::GATEWAY_IP
			};

			serial_println!("[PING] Resolving next hop MAC");
			super::arp::send_arp_request(next_hop)?;
			return Err("MAC not cached, ARP sent");
		}
	};

	let our_mac = super::get_our_mac().ok_or("No MAC")?;

	let icmp_data = b"Nullex Kernel Ping!";
	let total_len = 14 + 20 + 8 + icmp_data.len();

	let mut packet = alloc::vec![0u8; total_len];

	// ethernet - next hop MAC
	packet[0..6].copy_from_slice(&dst_mac);
	packet[6..12].copy_from_slice(&our_mac);
	packet[12..14].copy_from_slice(&super::ethernet::ETHERTYPE_IPV4.to_be_bytes());

	// IPv4 - actual destination IP
	packet[14] = 0x45;
	packet[15] = 0;
	let ip_total_len = (20 + 8 + icmp_data.len()) as u16;
	packet[16..18].copy_from_slice(&ip_total_len.to_be_bytes());
	packet[18..20].copy_from_slice(&1u16.to_be_bytes());
	packet[20..22].copy_from_slice(&0u16.to_be_bytes());
	packet[22] = 64;
	packet[23] = super::ipv4::IP_PROTO_ICMP;
	packet[26..30].copy_from_slice(&super::OUR_IP);
	packet[30..34].copy_from_slice(&dst_ip); // Actual destination!

	let ip_checksum = calculate_checksum(&packet[14..34]);
	packet[24..26].copy_from_slice(&ip_checksum.to_be_bytes());

	// ICMP
	packet[34] = ICMP_ECHO_REQUEST;
	packet[35] = 0;
	packet[38..40].copy_from_slice(&1u16.to_be_bytes());
	packet[40..42].copy_from_slice(&sequence.to_be_bytes());
	packet[42..].copy_from_slice(icmp_data);

	let icmp_checksum = calculate_checksum(&packet[34..]);
	packet[36..38].copy_from_slice(&icmp_checksum.to_be_bytes());

	super::send_packet(&packet)?;
	serial_println!(
		"[PING] Sent to {}.{}.{}.{} (seq={}) via next hop",
		dst_ip[0],
		dst_ip[1],
		dst_ip[2],
		dst_ip[3],
		sequence
	);
	Ok(())
}
