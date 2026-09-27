use x86_64::{
	PhysAddr,
	registers::control::{Cr3, Cr3Flags},
	structures::paging::PhysFrame
};

// raii guard.
pub struct KernelCr3Guard(Option<(PhysFrame, Cr3Flags)>);

impl KernelCr3Guard {
	#[inline(always)]
	pub unsafe fn enter_interrupt() -> Self {
		let current = Cr3::read();
		let kernel_cr3 = unsafe { crate::arch::x86_64::user::KERNEL_CR3 };

		if kernel_cr3 != 0 && current.0.start_address().as_u64() != kernel_cr3 {
			unsafe {
				Cr3::write(
					PhysFrame::containing_address(PhysAddr::new(kernel_cr3)),
					current.1
				)
			};
			Self(Some(current))
		} else {
			Self(None)
		}
	}
}

impl Drop for KernelCr3Guard {
	#[inline(always)]
	fn drop(&mut self) {
		if let Some((frame, flags)) = self.0 {
			unsafe { Cr3::write(frame, flags) }
		}
	}
}

pub unsafe fn enter_kernel_from_interrupt() -> Option<(PhysFrame, Cr3Flags)> {
	let current = Cr3::read();
	let kernel_cr3 = unsafe { crate::arch::x86_64::user::KERNEL_CR3 };
	if kernel_cr3 != 0 && current.0.start_address().as_u64() != kernel_cr3 {
		unsafe {
			Cr3::write(
				PhysFrame::containing_address(PhysAddr::new(kernel_cr3)),
				current.1
			)
		};
		Some(current)
	} else {
		None
	}
}

pub unsafe fn exit_kernel_from_interrupt(saved: Option<(PhysFrame, Cr3Flags)>) {
	if let Some((frame, flags)) = saved {
		unsafe { Cr3::write(frame, flags) };
	}
}
