use core::sync::atomic::Ordering;

use crate::{
	arch::x86_64::user::{
		KERNEL_CR3,
		KERNEL_RETURN_ADDR,
		KERNEL_RETURN_RBP,
		KERNEL_RETURN_RSP,
		USER_EXIT_CODE,
		USER_EXIT_REQUESTED
	},
	println
};

pub fn sys_halt(exit_code: i32) -> ! {
	USER_EXIT_CODE.store(exit_code, Ordering::SeqCst);
	USER_EXIT_REQUESTED.store(true, Ordering::SeqCst);
	println!("process ended with exit code: {}", exit_code);

	unsafe {
		core::arch::asm!(
			"mov cr3, {cr3}",
			"mov rsp, [{krsp}]",
			"mov rbp, [{krbp}]",
			"jmp [{kret}]",
			cr3  = in(reg) KERNEL_CR3,
			krsp = in(reg) core::ptr::addr_of!(KERNEL_RETURN_RSP),
			krbp = in(reg) core::ptr::addr_of!(KERNEL_RETURN_RBP),
			kret = in(reg) core::ptr::addr_of!(KERNEL_RETURN_ADDR),
			options(noreturn)
		);
	}
}
