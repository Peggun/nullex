// src/userspace/src/main.rs

#![no_std]
#![no_main]
#![feature(lang_items)]

use core::{arch::asm, panic::PanicInfo};

use orchestrator::syscall_interface::{SYS_EXIT, SYS_PRINT};

/// Performs a syscall using the syscall instruction.
/// Defined here directly for simplicity, matching userspace/src/syscalls.rs.
unsafe fn syscall(id: u32, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i32 {
	let result: i32;
	unsafe {
		asm!(
			"syscall",
			in("rax") id,
			in("rdi") arg1,
			in("rsi") arg2,
			in("rdx") arg3,
			in("r10") arg4,
			in("r8") arg5,
			lateout("rax") result,
			clobber_abi("sysv64"),
		);
	}
	result
}

/// Entry point for the userspace program.
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
	let s = "Hello from real userspace lol!\n\0"; // Null-terminated for simplicity
	unsafe {
		syscall(SYS_PRINT, s.as_ptr() as u64, s.len() as u64, 0, 0, 0);
		syscall(SYS_EXIT, 0, 0, 0, 0, 0);
	}
	loop {} // Unreachable, but required since we don’t return
}

/// Panic handler.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
