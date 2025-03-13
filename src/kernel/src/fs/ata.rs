// ata.rs

/*
ATA disk module for the kernel.
*/

use x86_64::instructions::{interrupts, port::Port};

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

	pub fn wait_ready(&mut self) -> Result<(), &'static str> {
		let mut timeout = 100_000;
		unsafe {
			while timeout > 0 {
				let status = self.status_port.read();
				if status & 0x80 == 0 {
					// BSY clear
					if status & 0x21 != 0 {
						// Check ERR/DF
						return Err("Drive error");
					}
					return Ok(());
				}
				timeout -= 1;
			}
		}
		Err("Timeout waiting for drive")
	}

	pub fn read_sector(&mut self, lba: u32, buf: &mut [u8; 512]) -> Result<(), &'static str> {
		interrupts::without_interrupts(|| {
			unsafe {
				// 1. Select SLAVE drive (second disk in QEMU)
				self.device_port.write(0xF0 | ((lba >> 24) as u8 & 0x0F));

				// 2. Full sector read
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
