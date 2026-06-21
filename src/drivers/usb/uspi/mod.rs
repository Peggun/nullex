//!
//! src/drivers/usb/uspi/mod.rs
//!
//! Full Rust port of the USPi USB driver.

use ruspiro_mailbox::Mailbox;

pub mod bcm2835;
#[allow(non_upper_case_globals, non_snake_case)]
pub mod devicenameservice;
#[allow(non_upper_case_globals, non_snake_case)]
pub mod dwhci;
#[allow(non_upper_case_globals, non_snake_case)]
pub mod os;
#[allow(non_upper_case_globals, non_snake_case)]
pub mod synchronize;
#[allow(non_upper_case_globals, non_snake_case)]
pub mod usb;

/// Raspberry Pi board model IDs returned by the VideoCore GPU
/// using the mailbox property interface tag `0x00010001`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RaspberryPiModel {
	ModelA = 0,
	ModelB = 1,
	ModelAPlus = 2,
	ModelBPlus = 3,
	Pi2ModelB = 4,
	Alpha = 5,
	ComputeModule1 = 6,
	Pi2ModelA = 7,
	Pi3ModelB = 8,
	PiZero = 9,
	ComputeModule3 = 0xA,
	PiZeroW = 0xC,
	Pi3ModelBPlus = 0xD
}

impl TryFrom<u32> for RaspberryPiModel {
	type Error = &'static str;

	fn try_from(value: u32) -> Result<Self, Self::Error> {
		match value {
			0 => Ok(RaspberryPiModel::ModelA),
			1 => Ok(RaspberryPiModel::ModelB),
			2 => Ok(RaspberryPiModel::ModelAPlus),
			3 => Ok(RaspberryPiModel::ModelBPlus),
			4 => Ok(RaspberryPiModel::Pi2ModelB),
			5 => Ok(RaspberryPiModel::Alpha),
			6 => Ok(RaspberryPiModel::ComputeModule1),
			7 => Ok(RaspberryPiModel::Pi2ModelA),
			8 => Ok(RaspberryPiModel::Pi3ModelB),
			9 => Ok(RaspberryPiModel::PiZero),
			0xA => Ok(RaspberryPiModel::ComputeModule3),
			0xC => Ok(RaspberryPiModel::PiZeroW),
			0xD => Ok(RaspberryPiModel::Pi3ModelBPlus),
			_ => Err("Unknown Raspberry Pi board model ID")
		}
	}
}

pub fn get_rpi_model() -> RaspberryPiModel {
	let mut mb = Mailbox::new();
	let model = mb.get_board_model().unwrap();
	RaspberryPiModel::try_from(model).unwrap()
}

impl RaspberryPiModel {
	/// Returns true when the model is from the Raspberry Pi 1 family
	pub fn is_rpi1(&self) -> bool {
		matches!(
			self,
			RaspberryPiModel::ModelA
				| RaspberryPiModel::ModelB
				| RaspberryPiModel::ModelAPlus
				| RaspberryPiModel::ModelBPlus
		)
	}

	/// Returns true when the model is from the Raspberry Pi 2 family
	pub fn is_rpi2(&self) -> bool {
		matches!(
			self,
			RaspberryPiModel::Pi2ModelB | RaspberryPiModel::Pi2ModelA
		)
	}

	/// Returns true when the model is from the Raspberry Pi 3 family
	pub fn is_rpi3(&self) -> bool {
		matches!(
			self,
			RaspberryPiModel::Pi3ModelB | RaspberryPiModel::Pi3ModelBPlus
		)
	}

	/// Returns true when the model is a Zero variant
	pub fn is_zero(&self) -> bool {
		matches!(self, RaspberryPiModel::PiZero | RaspberryPiModel::PiZeroW)
	}

	/// Returns true when the model is a Compute Module
	pub fn is_compute_module(&self) -> bool {
		matches!(
			self,
			RaspberryPiModel::ComputeModule1 | RaspberryPiModel::ComputeModule3
		)
	}
}
