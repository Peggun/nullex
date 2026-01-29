use crate::serial_println;

pub const ETHERTYPE_ARP: u16 = 0x0806;
pub const ETHERTYPE_IPV4: u16 = 0x0800;

pub fn process_ethernet_frame(ptr: *const u8, len: usize) {
	if len < 14 {
		serial_println!("[ETH] Too short");
		return;
	}

	unsafe {
		let ethertype = u16::from_be_bytes([*ptr.add(12), *ptr.add(13)]);
		let src_mac = [
			*ptr.add(6),
			*ptr.add(7),
			*ptr.add(8),
			*ptr.add(9),
			*ptr.add(10),
			*ptr.add(11)
		];

		serial_println!("[ETH] type={:#06x}", ethertype);

		match ethertype {
			ETHERTYPE_ARP => super::arp::process_arp(ptr, len, src_mac),
			ETHERTYPE_IPV4 => super::ipv4::process_ipv4(ptr, len),
			_ => serial_println!("[ETH] Unknown type")
		}
	}
}
