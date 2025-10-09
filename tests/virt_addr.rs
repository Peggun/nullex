#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nullex::test_runner)]
#![reexport_test_harness_main = "test_main"]

use core::panic::PanicInfo;

use nullex::{arch::x86_64::addr::VirtAddr, exit_qemu, serial_print, serial_println, QemuExitCode};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    serial_print!("virt_addr::virt_addr...\t");

    

    serial_println!("[ok]");
	exit_qemu(QemuExitCode::Success);
	loop {
		x86_64::instructions::hlt();
	}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	nullex::test_panic_handler(info)
}