use alloc::{collections::BTreeMap, string::String, vec::Vec};

use crate::{lazy_static, serial_println, utils::mutex::SpinMutex};

// DNS server (QEMU Default)
// quick note here. 10.0.2.3 is the usermode DNS address
// however we are using TAP currently, so we send all requests to
// to the gateway (10.0.2.2)
pub const DNS_SERVER: [u8; 4] = [10, 0, 2, 2];

pub const DNS_TIMEOUT_MS: u32 = 5000;

pub const DNS_POLL_INTERVAL_MS: u32 = 50;

lazy_static! {
	pub static ref DNS_CACHE: SpinMutex<Vec<(String, [u8; 4])>> = SpinMutex::new(Vec::new());
	pub static ref PENDING_QUERIES: SpinMutex<BTreeMap<u16, String>> =
		SpinMutex::new(BTreeMap::new());
	pub static ref DNS_RESPONSES: SpinMutex<BTreeMap<u16, Option<[u8; 4]>>> =
		SpinMutex::new(BTreeMap::new());
	pub static ref QUERY_ID_COUNTER: SpinMutex<u16> = SpinMutex::new(1000);
}

pub fn init() {
	super::udp::register_handler(53, handle_dns_response);
	serial_println!("[DNS] Initialized");
}

pub fn resolve(hostname: &str) -> Result<[u8; 4], &'static str> {
	{
		let cache = DNS_CACHE.lock();
		if let Some((_, ip)) = cache.iter().find(|(name, _)| name == hostname) {
			serial_println!(
				"[DNS] Cache hit: {} -> {}.{}.{}.{}",
				hostname,
				ip[0],
				ip[1],
				ip[2],
				ip[3]
			);
			return Ok(*ip);
		}
	}

	serial_println!("[DNS] Resolving {}...", hostname);
	let query_id = send_dns_query(hostname)?;

	wait_for_dns_response(query_id, hostname)
}

fn wait_for_dns_response(query_id: u16, hostname: &str) -> Result<[u8; 4], &'static str> {
	let poll_interval = 10; // ms
	let max_iterations = DNS_TIMEOUT_MS / poll_interval;

	for iteration in 0..max_iterations {
		{
			let mut responses = DNS_RESPONSES.lock();
			if let Some(Some(ip)) = responses.remove(&query_id) {
				serial_println!(
					"[DNS] Got response for {}: {}.{}.{}.{}",
					hostname,
					ip[0],
					ip[1],
					ip[2],
					ip[3]
				);
				return Ok(ip);
			}
		}

		crate::drivers::virtio::net::rx_poll();

		for _ in 0..100000 {
			core::hint::spin_loop();
		}

		if iteration % 50 == 0 && iteration > 0 {
			serial_println!(
				"[DNS] Still waiting for response ({}/{}ms)",
				iteration * poll_interval,
				DNS_TIMEOUT_MS
			);
		}
	}

	serial_println!("[DNS] Timeout resolving {}", hostname);
	let mut responses = DNS_RESPONSES.lock();
	responses.remove(&query_id);
	let mut pending = PENDING_QUERIES.lock();
	pending.remove(&query_id);
	Err("DNS timeout")
}

fn send_dns_query(hostname: &str) -> Result<u16, &'static str> {
	use alloc::string::ToString;

	let transaction_id = {
		let mut counter = QUERY_ID_COUNTER.lock();
		let id = *counter;
		*counter = counter.wrapping_add(1);
		id
	};

	{
		let mut pending = PENDING_QUERIES.lock();
		pending.insert(transaction_id, hostname.to_string());
		let mut responses = DNS_RESPONSES.lock();
		responses.insert(transaction_id, None);
	}

	let mut query = Vec::new();

	// DNS header
	query.extend_from_slice(&transaction_id.to_be_bytes());
	query.extend_from_slice(&0x0100u16.to_be_bytes());
	query.extend_from_slice(&0x0001u16.to_be_bytes());
	query.extend_from_slice(&0x0000u16.to_be_bytes());
	query.extend_from_slice(&0x0000u16.to_be_bytes());
	query.extend_from_slice(&0x0000u16.to_be_bytes());

	// question section
	for part in hostname.split('.') {
		query.push(part.len() as u8);
		query.extend_from_slice(part.as_bytes());
	}
	query.push(0);

	query.extend_from_slice(&0x0001u16.to_be_bytes());
	query.extend_from_slice(&0x0001u16.to_be_bytes());

	// Ensure gateway MAC is resolved and cached for DNS_SERVER
	let gateway_mac = if let Some(mac) = super::arp::get_cached(super::GATEWAY_IP) {
		serial_println!("[DNS] Using cached gateway MAC");
		mac
	} else {
		serial_println!("[DNS] Resolving gateway MAC...");
		super::arp::send_arp_request(super::GATEWAY_IP)?;

		match super::arp::wait_for_arp(super::GATEWAY_IP, 5000) {
			Ok(mac) => {
				serial_println!("[DNS] Gateway MAC resolved");
				mac
			}
			Err(e) => {
				serial_println!("[DNS] Failed to resolve gateway MAC: {}", e);
				let mut pending = PENDING_QUERIES.lock();
				pending.remove(&transaction_id);
				let mut responses = DNS_RESPONSES.lock();
				responses.remove(&transaction_id);
				return Err("Failed to resolve gateway MAC");
			}
		}
	};

	{
		let mut cache = super::arp::ARP_CACHE.lock();
		// Remove old entry if exists
		cache.retain(|(ip, _)| ip != &DNS_SERVER);
		cache.push((DNS_SERVER, gateway_mac));
		serial_println!("[DNS] Cached DNS server IP with gateway MAC");
	}

	match super::udp::send_udp(DNS_SERVER, 12345, 53, &query) {
		Ok(()) => {
			serial_println!("[DNS] Query sent for {} (id={})", hostname, transaction_id);
			Ok(transaction_id)
		}
		Err(e) => {
			serial_println!("[DNS] Failed to send DNS query: {}", e);
			let mut pending = PENDING_QUERIES.lock();
			pending.remove(&transaction_id);
			let mut responses = DNS_RESPONSES.lock();
			responses.remove(&transaction_id);
			Err(e)
		}
	}
}

fn handle_dns_response(payload: &[u8]) {
	if payload.len() < 12 {
		serial_println!("[DNS] Response too short");
		return;
	}

	let transaction_id = u16::from_be_bytes([payload[0], payload[1]]);
	let flags = u16::from_be_bytes([payload[2], payload[3]]);
	let questions = u16::from_be_bytes([payload[4], payload[5]]);
	let answers = u16::from_be_bytes([payload[6], payload[7]]);

	serial_println!(
		"[DNS] Response: id={:#x}, flags={:#x}, answers={}",
		transaction_id,
		flags,
		answers
	);

	// is this a response? (QR bit set)
	if (flags & 0x8000) == 0 {
		serial_println!("[DNS] Not a response");
		return;
	}

	// find pending query
	let hostname = {
		let mut pending = PENDING_QUERIES.lock();
		pending.remove(&transaction_id)
	};

	let hostname = match hostname {
		Some(h) => h,
		None => {
			serial_println!("[DNS] Unknown transaction ID {}", transaction_id);
			return;
		}
	};

	// parsing
	let mut offset = 12;

	// skip question section
	for _ in 0..questions {
		// skip QNAME
		while offset < payload.len() && payload[offset] != 0 {
			let len = payload[offset] as usize;
			offset += 1 + len;
		}
		offset += 1; // skip null terminator
		offset += 4; // skip QTYPE and QCLASS
	}

	// parse answer section
	for _ in 0..answers {
		if offset + 12 > payload.len() {
			break;
		}

		// skip NAME (might be compressed)
		if (payload[offset] & 0xC0) == 0xC0 {
			offset += 2; // compressed name pointer
		} else {
			while offset < payload.len() && payload[offset] != 0 {
				let len = payload[offset] as usize;
				offset += 1 + len;
			}
			offset += 1;
		}

		let rtype = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
		let rdlength = u16::from_be_bytes([payload[offset + 8], payload[offset + 9]]);
		offset += 10; // skip others like TYPE, CLASS, TTL, RDLENGTH

		if rtype == 1 && rdlength == 4 {
			// type A (IPv4 address)
			let ip = [
				payload[offset],
				payload[offset + 1],
				payload[offset + 2],
				payload[offset + 3]
			];

			serial_println!(
				"[DNS] Resolved {} -> {}.{}.{}.{}",
				hostname,
				ip[0],
				ip[1],
				ip[2],
				ip[3]
			);

			{
				let mut cache = DNS_CACHE.lock();
				cache.push((hostname.clone(), ip));
			}

			{
				let mut responses = DNS_RESPONSES.lock();
				responses.insert(transaction_id, Some(ip));
			}

			return;
		}

		offset += rdlength as usize;
	}

	serial_println!("[DNS] No A record found in response");
	let mut responses = DNS_RESPONSES.lock();
	responses.remove(&transaction_id);
}

pub fn get_cached(hostname: &str) -> Option<[u8; 4]> {
	let cache = DNS_CACHE.lock();
	cache
		.iter()
		.find(|(name, _)| name == hostname)
		.map(|(_, ip)| *ip)
}
