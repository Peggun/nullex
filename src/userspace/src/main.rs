#![no_main]
#![no_std]
#[allow(asm_sub_register)]

use core::{arch::asm, panic::PanicInfo};

const SYS_PRINT: u32 = 0;
const SYS_EXIT: u32 = 1;

#[unsafe(no_mangle)]
pub extern "C" fn _start() {
	let message = "Hello from userspace!\n";
	unsafe {
		asm!(
			"mov eax, {id}",      // Syscall ID
			"mov rdi, {arg1}",    // Arg 1: pointer to string
			"mov rsi, {arg2}",    // Arg 2: length
			"syscall",
			id = const SYS_PRINT,
			arg1 = in(reg) message.as_ptr(),
			arg2 = in(reg) message.len(),
			out("rax") _,
			out("rcx") _,
			out("r11") _,
		);
	}

	unsafe {
		asm!(
			"mov eax, {id}",      // Syscall ID
			"mov rdi, {arg1}",    // Arg 1: exit code
			"syscall",
			id = const SYS_EXIT,
			arg1 = in(reg) 42,
			out("rax") _,
			out("rcx") _,
			out("r11") _,
		);
	}
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
	loop {}
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
	loop {}
}
