//!
//! ipv4.rs
//! 
//! IPv4 packet handling logic for the kernel.
//! 

use crate::serial_println;

/// ICMP IP Protocol Value
pub const IP_PROTO_ICMP: u8 = 1;
/// TCP IP Protocol Value
pub const IP_PROTO_TCP: u8 = 6;
/// UDP IP Protocol Value
pub const IP_PROTO_UDP: u8 = 17;

/// Process incoming IPv4 packets.
pub fn process_ipv4(pkt: *const u8, len: usize) {
	if len < 34 {
		serial_println!("[IPv4] Packet too short: {} bytes", len);
		return;
	}

	unsafe {
		let ip_start = pkt.add(14);

		let version_ihl = *ip_start;
		let version = version_ihl >> 4;
		let ihl = (version_ihl & 0x0F) as usize * 4;

		if version != 4 {
			serial_println!("[IPv4] Not IPv4: version {}", version);
			return;
		}

		let protocol = *ip_start.add(9);
		let src_ip = [
			*ip_start.add(12),
			*ip_start.add(13),
			*ip_start.add(14),
			*ip_start.add(15)
		];
		let dst_ip = [
			*ip_start.add(16),
			*ip_start.add(17),
			*ip_start.add(18),
			*ip_start.add(19)
		];

		serial_println!(
			"[IPv4] src={}.{}.{}.{}, dst={}.{}.{}.{}, proto={}",
			src_ip[0],
			src_ip[1],
			src_ip[2],
			src_ip[3],
			dst_ip[0],
			dst_ip[1],
			dst_ip[2],
			dst_ip[3],
			protocol
		);

		if dst_ip != super::OUR_IP {
			serial_println!("[IPv4] Not for us, dropping");
			return;
		}

		match protocol {
			IP_PROTO_ICMP => {
				super::icmp::process_icmp(pkt, len, ihl, &src_ip);
			}
			IP_PROTO_TCP => {
				serial_println!("[IPv4] TCP not implemented");
			}
			IP_PROTO_UDP => {
				super::udp::process_udp(pkt, len, ihl, &src_ip);
			}
			_ => {
				serial_println!("[IPv4] Unknown protocol: {}", protocol);
			}
		}
	}
}
