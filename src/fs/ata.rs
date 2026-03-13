//! 
//! ata.rs
//!
//! ATA disk module for the kernel.
//!
 
//
// Currently not in use. But for the future.
//

use x86_64::instructions::{interrupts, port::Port};

use crate::error::NullexError;

pub struct AtaDisk {
	data_port: Port<u16>,
	pub sector_count_port: Port<u8>,
	pub lba_low_port: Port<u8>,
	pub lba_mid_port: Port<u8>,
	pub lba_high_port: Port<u8>,
	pub device_port: Port<u8>,
	pub command_port: Port<u8>,
	pub status_port: Port<u8>
}

impl AtaDisk {
	/// # Safety
	/// TODO: why is this unsafe?
	pub unsafe fn new() -> Self {
		AtaDisk {
			data_port: Port::new(0x1F0),
			sector_count_port: Port::new(0x1F2),
			lba_low_port: Port::new(0x1F3),
			lba_mid_port: Port::new(0x1F4),
			lba_high_port: Port::new(0x1F5),
			device_port: Port::new(0x1F6),
			command_port: Port::new(0x1F7),
			status_port: Port::new(0x1F7)
		}
	}

	pub fn wait_ready(&mut self) -> Result<(), NullexError> {
		let mut timeout = 100_000;
		unsafe {
			while timeout > 0 {
				let status = self.status_port.read();
				if status & 0x80 == 0 {
					// BSY clear
					if status & 0x21 != 0 {
						// check ERR/DF
						return Err(NullexError::AtaDriveError);
					}
					return Ok(());
				}
				timeout -= 1;
			}
		}
		Err(NullexError::AtaTimeout)
	}

	pub fn read_sector(&mut self, lba: u32, buf: &mut [u8; 512]) -> Result<(), NullexError> {
		interrupts::without_interrupts(|| {
			unsafe {
				// select `slave` drive (second disk in QEMU)
				self.device_port.write(0xF0 | ((lba >> 24) as u8 & 0x0F));

				// full sector read
				for i in 0..256 {
					let word = self.data_port.read();
					buf[i * 2] = word as u8;
					buf[i * 2 + 1] = (word >> 8) as u8;
				}
				Ok(())
			}
		})
	}
}
