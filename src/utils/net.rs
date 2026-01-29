/// Calculate Internet checksum (RFC 1071)
pub fn calculate_checksum(data: &[u8]) -> u16 {
	let mut sum: u32 = 0;

	for chunk in data.chunks(2) {
		let word = if chunk.len() == 2 {
			u16::from_be_bytes([chunk[0], chunk[1]]) as u32
		} else {
			(chunk[0] as u32) << 8
		};
		sum += word;
	}

	while (sum >> 16) != 0 {
		sum = (sum & 0xFFFF) + (sum >> 16);
	}

	!sum as u16
}
