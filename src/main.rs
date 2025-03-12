// main.rs

/*
Main entry code for the kernel.

Note to self: you can use this to use the error handling
If needed:

let (Ok(my_var)|Err(my_var)) = foo(5);

https://stackoverflow.com/questions/76196072/get-value-t-from-resultt-t
*/

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nullex::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, sync::Arc};
use core::{
	sync::atomic::Ordering,
	task::{Context, Poll}
};

use bootloader::{BootInfo, entry_point};
use nullex::{
	allocator,
	apic::{self, apic::sleep},
	constants::{SYSLOG_SINK, initialize_constants},
	errors::KernelError,
	fs::{
		self,
		ramfs::{FileSystem, Permission}
	},
	interrupts::PICS,
	memory::{self, BootInfoFrameAllocator},
	println,
	serial_println,
	task::{
		Process,
		ProcessState,
		executor::{self, CURRENT_PROCESS, EXECUTOR},
		keyboard
	},
	utils::{
		logger::{levels::LogLevel, traits::logger_sink::LoggerSink},
		process::spawn_process
	},
	vga_buffer::WRITER
};
use raw_cpuid::CpuId;
use vga::colors::Color16;

entry_point!(kernel_main);

/// A dummy async delay approximating half a second.
async fn sleep_half_second() {
	unsafe {
		let _ = sleep(500).await;
	}
}

/// Process 1: prints a message every half second.
async fn process_one(_state: Arc<ProcessState>) -> Result<i32, KernelError> {
	loop {
		serial_println!("Process 1: Hello every half second");
		sleep_half_second().await;
	}
	//Ok(0) // Unreachable, but required for type consistency
}

/// Process 2: prints a message every half second.
async fn process_two(_state: Arc<ProcessState>) -> Result<i32, KernelError> {
	loop {
		serial_println!("Process 2: Hello every half second");
		sleep_half_second().await;
	}
	//Ok(0) // Unreachable, but required for type consistency
}

#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
	use x86_64::VirtAddr;

	println!("[Info] Starting Kernel Init...");

	let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
	let mut mapper = unsafe { memory::init(phys_mem_offset) };
	let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

	unsafe {
		PICS.lock().write_masks(0b11111101, 0b11111111);
	}

	nullex::init();

	match allocator::init_heap(&mut mapper, &mut frame_allocator) {
		Ok(()) => println!("Heap initialized successfully"),
		Err(e) => panic!("Heap initialization failed: {:?}", e)
	}

	unsafe {
		let _ = apic::apic::enable_apic();
	};
	memory::map_apic(&mut mapper, &mut frame_allocator);
	unsafe {
		let _ = apic::apic::init_timer(6125);
	};
	initialize_constants();

	let cpuid = CpuId::new();

	if let Some(vf) = cpuid.get_vendor_info() {
		serial_println!("Vendor Info: {}", vf.as_str())
	}

	let has_sse = cpuid
		.get_feature_info()
		.map_or(false, |finfo| finfo.has_sse());
	if has_sse {
		serial_println!("CPU supports SSE!");
	}

	if let Some(cparams) = cpuid.get_cache_parameters() {
		for cache in cparams {
			let size = cache.associativity()
				* cache.physical_line_partitions()
				* cache.coherency_line_size()
				* cache.sets();
			serial_println!("L{}-Cache size is {}", cache.level(), size);
		}
	} else {
		serial_println!("No cache parameter information available")
	}

	let mut fs = FileSystem::new();

	println!("[Info] Initializing RAMFS...");

	if let Err(e) = fs.create_dir("/logs", Permission::all()) {
		panic!("Failed to create /logs directory: {:?}", e);
	}

	fs::init_fs(fs);

	println!("[Info] Done.");

	SYSLOG_SINK.log("Initialized Main Kernel Successfully\n", LogLevel::Info);

	WRITER.lock().clear_everything();
	WRITER.lock().set_colors(Color16::White, Color16::Black);

	let _ = crate::keyboard::commands::init_commands();

	// Spawn the keyboard process.
	let _keyboard_pid = spawn_process(|_state| Box::pin(keyboard::print_keypresses()), false);

	// Spawn process one.
	let _process1_pid = spawn_process(|state| Box::pin(process_one(state)), false);

	// Spawn process two.
	let _process2_pid = spawn_process(|state| Box::pin(process_two(state)), false);

	// Main executor loop with CURRENT_PROCESS management.
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
							executor::ProcessWaker::new(pid, process_queue.clone(), process_state)
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
					// Handle Result<i32, KernelError> from all processes
					match exit_code {
						Ok(code) => {
							serial_println!("Process {} exited with code: {}", pid.get(), code)
						}
						Err(e) => {
							serial_println!("Process {} failed with error: {:?}", pid.get(), e)
						}
					}
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
