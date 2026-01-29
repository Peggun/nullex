use alloc::vec::Vec;

use x86_64::structures::idt::InterruptStackFrame;

use crate::{lazy_static, utils::mutex::SpinMutex};

#[derive(Debug, Default, Clone)]
pub struct GsiInfo {
	pub flags: u16,
	pub has_iso: bool,
	pub vector: Option<u8>,
	pub device_ptr: Option<usize>,
	pub handler: Option<extern "x86-interrupt" fn(InterruptStackFrame)>,

	pub pending: bool
}

lazy_static! {
	pub static ref GSI_TABLE: SpinMutex<Vec<GsiInfo>> =
		SpinMutex::new(vec![GsiInfo::default(); 256]);
}
