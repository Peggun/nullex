use alloc::vec::Vec;

use crate::{lazy_static, serial_println, utils::mutex::SpinMutex};

pub const ARP_OP_REQUEST: u16 = 1;
pub const ARP_OP_REPLY: u16 = 2;

lazy_static! {
	pub static ref ARP_CACHE: SpinMutex<Vec<([u8; 4], [u8; 6])>> = SpinMutex::new(Vec::new());
}

pub fn process_arp(pkt: *const u8, len: usize, _src_mac: [u8; 6]) {
	if len < 42 {
		serial_println!("[ARP] Packet too short: {} bytes", len);
		return;
	}

	unsafe {
		let arp_start = pkt.add(14);

		let operation = u16::from_be_bytes([*arp_start.add(6), *arp_start.add(7)]);
		let sender_mac = [
			*arp_start.add(8),
			*arp_start.add(9),
			*arp_start.add(10),
			*arp_start.add(11),
			*arp_start.add(12),
			*arp_start.add(13)
		];
		let sender_ip = [
			*arp_start.add(14),
			*arp_start.add(15),
			*arp_start.add(16),
			*arp_start.add(17)
		];
		let target_ip = [
			*arp_start.add(24),
			*arp_start.add(25),
			*arp_start.add(26),
			*arp_start.add(27)
		];

		serial_println!(
			"[ARP] Operation: {}, Sender: {}.{}.{}.{} -> {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
			operation,
			sender_ip[0],
			sender_ip[1],
			sender_ip[2],
			sender_ip[3],
			sender_mac[0],
			sender_mac[1],
			sender_mac[2],
			sender_mac[3],
			sender_mac[4],
			sender_mac[5]
		);

		match operation {
			ARP_OP_REQUEST => {
				{
					let mut cache = ARP_CACHE.lock();
					cache.retain(|(ip, _)| ip != &sender_ip);
					cache.push((sender_ip, sender_mac));
					serial_println!(
						"[ARP] Cached sender: {}.{}.{}.{}",
						sender_ip[0],
						sender_ip[1],
						sender_ip[2],
						sender_ip[3]
					);
				}

				// Check if request is for us
				if target_ip == super::OUR_IP {
					serial_println!("[ARP] Request for our IP, sending reply");
					send_arp_reply(&sender_mac, &sender_ip);
				}
			}
			ARP_OP_REPLY => {
				let mut cache = ARP_CACHE.lock();
				cache.retain(|(ip, _)| ip != &sender_ip);
				cache.push((sender_ip, sender_mac));
				serial_println!(
					"[ARP] Cached reply from {}.{}.{}.{}",
					sender_ip[0],
					sender_ip[1],
					sender_ip[2],
					sender_ip[3]
				);
			}
			_ => {
				serial_println!("[ARP] Unknown operation: {}", operation);
			}
		}
	}
}

fn send_arp_reply(target_mac: &[u8; 6], target_ip: &[u8; 4]) {
	let our_mac = match super::get_our_mac() {
		Some(mac) => mac,
		None => {
			serial_println!("[ARP] No MAC address");
			return;
		}
	};

	let mut packet = [0u8; 42];

	// ethernet header
	packet[0..6].copy_from_slice(target_mac);
	packet[6..12].copy_from_slice(&our_mac);
	packet[12..14].copy_from_slice(&super::ethernet::ETHERTYPE_ARP.to_be_bytes());

	// ARP packet
	packet[14..16].copy_from_slice(&1u16.to_be_bytes()); // HW type
	packet[16..18].copy_from_slice(&0x0800u16.to_be_bytes()); // Prototype
	packet[18] = 6; // HW len
	packet[19] = 4; // Proto len
	packet[20..22].copy_from_slice(&ARP_OP_REPLY.to_be_bytes());
	packet[22..28].copy_from_slice(&our_mac);
	packet[28..32].copy_from_slice(&super::OUR_IP);
	packet[32..38].copy_from_slice(target_mac);
	packet[38..42].copy_from_slice(target_ip);

	if let Err(e) = super::send_packet(&packet) {
		serial_println!("[ARP] Failed to send reply: {}", e);
	} else {
		serial_println!("[ARP] Reply sent");
	}
}

pub fn send_arp_request(target_ip: [u8; 4]) -> Result<(), &'static str> {
	let our_mac = super::get_our_mac().ok_or("No MAC address")?;

	let mut packet = [0u8; 42];

	// ethernet header (broadcast)
	packet[0..6].copy_from_slice(&[0xFF; 6]);
	packet[6..12].copy_from_slice(&our_mac);
	packet[12..14].copy_from_slice(&super::ethernet::ETHERTYPE_ARP.to_be_bytes());

	// ARP packet
	packet[14..16].copy_from_slice(&1u16.to_be_bytes());
	packet[16..18].copy_from_slice(&0x0800u16.to_be_bytes());
	packet[18] = 6;
	packet[19] = 4;
	packet[20..22].copy_from_slice(&ARP_OP_REQUEST.to_be_bytes());
	packet[22..28].copy_from_slice(&our_mac);
	packet[28..32].copy_from_slice(&super::OUR_IP);
	packet[32..38].copy_from_slice(&[0; 6]);
	packet[38..42].copy_from_slice(&target_ip);

	super::send_packet(&packet)?;
	serial_println!(
		"[ARP] Request sent for {}.{}.{}.{}",
		target_ip[0],
		target_ip[1],
		target_ip[2],
		target_ip[3]
	);
	Ok(())
}

pub fn wait_for_arp(ip: [u8; 4], timeout_ms: u32) -> Result<[u8; 6], &'static str> {
	let poll_interval = 10; // ms
	let max_iterations = timeout_ms / poll_interval;

	for iteration in 0..max_iterations {
		{
			let cache = ARP_CACHE.lock();
			for (cached_ip, cached_mac) in cache.iter() {
				if cached_ip == &ip {
					serial_println!(
						"[ARP] Found in cache: {}.{}.{}.{} -> {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
						ip[0],
						ip[1],
						ip[2],
						ip[3],
						cached_mac[0],
						cached_mac[1],
						cached_mac[2],
						cached_mac[3],
						cached_mac[4],
						cached_mac[5]
					);
					return Ok(*cached_mac);
				}
			}
		}

		crate::drivers::virtio::net::rx_poll();

		{
			let cache = ARP_CACHE.lock();
			for (cached_ip, cached_mac) in cache.iter() {
				if cached_ip == &ip {
					serial_println!(
						"[ARP] Found in cache after poll: {}.{}.{}.{} -> {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
						ip[0],
						ip[1],
						ip[2],
						ip[3],
						cached_mac[0],
						cached_mac[1],
						cached_mac[2],
						cached_mac[3],
						cached_mac[4],
						cached_mac[5]
					);
					return Ok(*cached_mac);
				}
			}
		}

		for _ in 0..100000 {
			core::hint::spin_loop();
		}

		if iteration % 50 == 0 {
			serial_println!(
				"[ARP] Still waiting for {}.{}.{}.{} ({}/{}ms)",
				ip[0],
				ip[1],
				ip[2],
				ip[3],
				iteration * poll_interval,
				timeout_ms
			);
		}
	}

	serial_println!(
		"[ARP] Timeout waiting for {}.{}.{}.{}",
		ip[0],
		ip[1],
		ip[2],
		ip[3]
	);
	Err("ARP timeout")
}

pub fn get_cached(ip: [u8; 4]) -> Option<[u8; 6]> {
	let cache = ARP_CACHE.lock();
	cache
		.iter()
		.find(|(cached_ip, _)| *cached_ip == ip)
		.map(|(_, mac)| *mac)
}
