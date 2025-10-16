// lib.rs

/*
Kernel module for the kernel.
*/

#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
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
extern crate spin;
extern crate libc;
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

use spin::mutex::Mutex;
use x86_64::VirtAddr;

use lazy_static::lazy_static;

use alloc::boxed::Box;
use core::{
	future::Future,
	pin::Pin,
	sync::atomic::Ordering,
	task::{Context, Poll}
};

use crate::{
	apic::write_register, constants::{initialize_constants, SYSLOG_SINK}, fs::ramfs::{FileSystem, Permission}, interrupts::{init_idt, PICS}, task::{
		executor::{self, CURRENT_PROCESS, EXECUTOR}, keyboard, Process
	}, utils::{
		logger::{levels::LogLevel, traits::logger_sink::LoggerSink},
		process::spawn_process
	}, vga_buffer::WRITER
};


lazy_static! {
	pub static ref PHYS_MEM_OFFSET: Mutex<VirtAddr> = Mutex::new(VirtAddr::new(0x0));
}

pub fn init() {
	serial_println!("[Info] Initializing kernel...");
	gdt::init();
	serial_println!("gdt done");
	interrupts::init_idt();
	serial_println!("[Info] Finished IDT Init...");
	unsafe { interrupts::PICS.lock().initialize() };
	x86_64::instructions::interrupts::enable();
	serial_println!("[Info] Done.");
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


#[unsafe(no_mangle)]
fn kernel_main() -> ! {
	WRITER.lock().clear_everything();
	println!("[Info] Starting Kernel Init...");


	// // let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
	// // let mut mapper = unsafe { memory::init(phys_mem_offset) };
	// // let mut frame_allocator = BootInfoFrameAllocator::init(&boot_info.memory_map);

	unsafe {
		PICS.lock().write_masks(0b11111101, 0b11111111);
	}

	crate::init();

	// // match allocator::init_heap(&mut mapper, &mut frame_allocator) {
	// // 	Ok(()) => println!("Heap initialized successfully"),
	// // 	Err(e) => panic!("Heap initialization failed: {:?}", e)
	// // }

	// // unsafe { apic::enable_apic() };
	// // memory::map_apic(&mut mapper, &mut frame_allocator);
	// // unsafe { apic::init_timer() };
	initialize_constants();

	let fs = FileSystem::new();

	println!("[Info] Initializing RAMFS...");

	// setup files and ramfs.
	setup_system_files(fs);

	println!("[Info] Done.");

	SYSLOG_SINK.log("Initialized Main Kernel Successfully\n", LogLevel::Info);

	WRITER.lock().clear_everything();
	// WRITER.lock().set_colors(Color16::White, Color16::Black);

	crate::keyboard::commands::init_commands();
	// init_serial_input();
	// init_serial_commands();

	// Spawn the keyboard process.
	let _keyboard_pid = spawn_process(
		|_state| Box::pin(keyboard::print_keypresses()) as Pin<Box<dyn Future<Output = i32>>>,
		false
	);

	// main executor loop with CURRENT_PROCESS management.
	// i gotta fix this.
	let process_queue = EXECUTOR.lock().process_queue.clone();
	loop {
		if let Some(pid) = process_queue.pop() {
			// Before scheduling, clear the queued flag.
			if let Some(process_arc) = EXECUTOR.lock().processes.get(&pid) {
				process_arc
					.lock()
					.state
					.queued
					.store(false, Ordering::Release);
			}

			let process_arc = {
				let executor = EXECUTOR.lock();
				executor.processes.get(&pid).cloned()
			};
			if let Some(process_arc) = process_arc {
				// Set the current process state.
				*CURRENT_PROCESS.lock() = Some(process_arc.lock().state.clone());

				let mut process = process_arc.lock();
				let process_state = process.state.clone(); // Clone the Arc<ProcessState> for the waker
				unsafe {
					executor::CURRENT_PROCESS_GUARD = &mut *process as *mut Process;
				}
				let waker = {
					let mut executor = EXECUTOR.lock();
					executor
						.waker_cache
						.entry(pid)
						.or_insert_with(|| {
							executor::ProcessWaker::new_waker(
								pid,
								process_queue.clone(),
								process_state
							)
						})
						.clone()
				};
				let mut context = Context::from_waker(&waker);
				let result = process.future.as_mut().poll(&mut context);
				unsafe {
					executor::CURRENT_PROCESS_GUARD = core::ptr::null_mut();
				}
				if let Poll::Ready(exit_code) = result {
					let mut executor = EXECUTOR.lock();
					executor.processes.remove(&pid);
					executor.waker_cache.remove(&pid);
					serial_println!("Process {} exited with code: {}", pid.get(), exit_code);
				}
				// Clear the current process state.
				*CURRENT_PROCESS.lock() = None;
			}
		} else {
			EXECUTOR.lock().sleep_if_idle();
		}
	}
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
	println!("{}", info);
	crate::hlt_loop();
}