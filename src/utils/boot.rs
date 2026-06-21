//!
//! boot.rs
//!
//! Boot-time module for the kernel.

use ::x86_64::registers::model_specific::{Efer, EferFlags};
use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};

/// Initialises the EFER register to allow for x86_64 NO_EXECUTE page table
/// flags.
pub fn init_efer() {
	unsafe {
		Efer::update(|flags| {
			*flags |= EferFlags::NO_EXECUTE_ENABLE;
		})
	}
}

pub fn enable_sse() {
	unsafe {
		let mut cr0 = Cr0::read();
		cr0.remove(Cr0Flags::EMULATE_COPROCESSOR);
		cr0.insert(Cr0Flags::MONITOR_COPROCESSOR);
		Cr0::write(cr0);

		let mut cr4 = Cr4::read();
		cr4.insert(Cr4Flags::OSFXSR);
		cr4.insert(Cr4Flags::OSXMMEXCPT_ENABLE);
		Cr4::write(cr4);
	}
}
