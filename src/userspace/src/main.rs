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
	unsafe {
		asm!(
			"mov eax, 1",
			"mov ebx, {msg}",
			"mov ecx, {len}",
			"int 0x80",
			"mov eax, 2",
			"mov ebx, 0",
			"int 0x80",
			msg = sym MSG,
			len = const MSG_LEN,
		);
	}
	loop {}
}

static MSG: &[u8] = b"Hello from userspace\0";
const MSG_LEN: usize = MSG.len();

/// Panic handler.
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
	loop {}
}

#[lang = "eh_personality"]
extern "C" fn eh_personality() {}
