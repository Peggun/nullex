// main.rs

/*
Main entry code for the kernel.
*/

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nullex::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::boxed::Box;
use core::{
	future::Future,
	pin::Pin,
	sync::atomic::Ordering,
	task::{Context, Poll}
};

use bootloader::{BootInfo, entry_point};
use nullex::{
	allocator, apic, arch::x86_64::addr::VirtAddr, constants::{initialize_constants, SYSLOG_SINK}, fs::ramfs::FileSystem, interrupts::{init_idt, PICS}, memory::{self, BootInfoFrameAllocator}, println, serial, serial_println, setup_system_files, task::{
		executor::{self, CURRENT_PROCESS, EXECUTOR}, keyboard, Process
	}, utils::{
		logger::{levels::LogLevel, traits::logger_sink::LoggerSink},
		process::spawn_process
	}, vga_buffer::WRITER
};

entry_point!(kernel_main);

#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
	println!("[Info] Starting Kernel Init...");
	init_idt();

	let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
	let mut mapper = unsafe { memory::init(phys_mem_offset) };
	let mut frame_allocator = BootInfoFrameAllocator::init(&boot_info.memory_map);

	unsafe {
		PICS.lock().write_masks(0b11111101, 0b11111111);
	}

	nullex::init();

	match allocator::init_heap(&mut mapper, &mut frame_allocator) {
		Ok(()) => println!("Heap initialized successfully"),
		Err(e) => panic!("Heap initialization failed: {:?}", e)
	}

	unsafe { apic::enable_apic() };
	memory::map_apic(&mut mapper, &mut frame_allocator);
	unsafe { apic::init_timer() };
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
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
	println!("{}", info);
	nullex::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
	println!("{}", info);
	nullex::hlt_loop();
}
