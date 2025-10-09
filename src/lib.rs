// lib.rs

/*
Kernel module for the kernel.
*/

#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(step_trait)]
#![feature(associated_type_defaults)]
#![feature(alloc_error_handler)]
#![feature(str_from_raw_parts)]
#![feature(generic_atomic)]
#![feature(string_from_utf8_lossy_owned)]

#[macro_use]
extern crate alloc;
extern crate bitflags;
extern crate libc;
extern crate spin;

#[cfg(test)]
extern crate core;

pub mod allocator;
pub mod apic;
pub mod common;
pub mod config;
pub mod constants;
pub mod error;
pub mod fs;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod pit;
pub mod programs;
pub mod serial;
pub mod syscall;
pub mod task;
pub mod utils;
pub mod vga_buffer;

pub mod arch;

use core::panic::PanicInfo;

#[cfg(test)]
use bootloader::{BootInfo, entry_point};
use spin::mutex::Mutex;

use crate::{arch::x86_64::addr::VirtAddr, fs::ramfs::{FileSystem, Permission}};
use lazy_static::lazy_static;

lazy_static! {
	pub static ref PHYS_MEM_OFFSET: Mutex<VirtAddr> = Mutex::new(VirtAddr::new(0x0));
}

#[cfg(test)]
entry_point!(test_kernel_main);

pub trait Testable {
	fn run(&self) -> ();
}

impl<T> Testable for T
where
	T: Fn()
{
	fn run(&self) {
		serial_print!("{}...\t", core::any::type_name::<T>());
		self();
		serial_println!("[ok]");
	}
}

pub fn test_runner(tests: &[&dyn Testable]) {
	serial_println!("Running {} tests", tests.len());
	for test in tests {
		test.run();
	}
	exit_qemu(QemuExitCode::Success);
}

pub fn test_panic_handler(info: &PanicInfo) -> ! {
	serial_println!("[failed]\n");
	serial_println!("Error: {}\n", info);
	exit_qemu(QemuExitCode::Failed);
	hlt_loop();
}

/// Entry point for `cargo test`
#[cfg(test)]
fn test_kernel_main(_boot_info: &'static BootInfo) -> ! {
	// like before
	init();
	test_main();
	hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	test_panic_handler(info)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
	Success = 0x10,
	Failed = 0x11
}

pub fn exit_qemu(exit_code: QemuExitCode) {
	use x86_64::instructions::port::Port;

	unsafe {
		let mut port = Port::new(0xf4);
		port.write(exit_code as u32);
	}
}

pub fn init() {
	println!("[Info] Initializing kernel...");
	gdt::init();
	interrupts::init_idt();
	unsafe { interrupts::PICS.lock().initialize() };
	x86_64::instructions::interrupts::enable();
	println!("[Info] Done.");
}

pub fn hlt_loop() -> ! {
	loop {
		x86_64::instructions::hlt();
	}
}

#[repr(align(512))]
pub struct Align512<T>(T);
pub fn align_buffer(buffer: [u8; 512]) -> Align512<[u8; 512]> {
	Align512(buffer)
}

impl<T> Align512<T> {
	pub fn inner(&self) -> &T {
		&self.0
	}

	pub fn inner_mut(&mut self) -> &mut T {
		&mut self.0
	}
}

pub fn setup_system_files(mut fs: FileSystem) {
	fs.create_dir("/logs", Permission::all()).unwrap();
	fs.create_dir("/proc", Permission::read()).unwrap();

	fs.create_file("test.nx", Permission::all()).unwrap();

	fs.write_file(
		"test.nx",
		b"// simple test
func main() {
	set num = 1;

	print(\"Hello, world!\");
	print(num);
}",
		false
	)
	.unwrap();

	fs::init_fs(fs);
}
