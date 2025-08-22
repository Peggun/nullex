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

use alloc::{boxed::Box, sync::Arc};
use core::{
	future::Future,
	pin::Pin,
	sync::atomic::Ordering,
	task::{Context, Poll}
};

use bootloader::{BootInfo, entry_point};
use lazy_static::lazy_static;
use nullex::{
	allocator,
	apic::apic::{self, sleep},
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
		logger::{
			format::DefaultFormatter,
			sinks::{stdout::StdOutSink, syslog::SyslogSink}
		},
		process::spawn_process
	},
	vga_buffer::WRITER
};
use vga::colors::Color16;

lazy_static! {
	pub static ref STDOUT_SINK: StdOutSink = StdOutSink::new(Box::new(DefaultFormatter::new(true)));
	pub static ref SYSLOG_SINK: SyslogSink = SyslogSink::new(Box::new(DefaultFormatter::new(true)));
}

entry_point!(kernel_main);

/// A dummy async delay approximating half a second.
async fn sleep_half_second() {
	unsafe {
		sleep(500).await;
	}
}

/// Process 1: prints a message every half second.
async fn process_one(_state: Arc<ProcessState>) -> i32 {
	loop {
		serial_println!("Process 1: Hello every half second");
		sleep_half_second().await;
	}
}

/// Process 2: prints a message every half second.
async fn process_two(_state: Arc<ProcessState>) -> i32 {
	loop {
		serial_println!("Process 2: Hello every half second");
		sleep_half_second().await;
	}
}

#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
	use x86_64::VirtAddr;

	let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
	let mut mapper = unsafe { memory::init(phys_mem_offset) };
	let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

	unsafe { apic::enable_apic() };
	memory::map_apic(&mut mapper, &mut frame_allocator);
	unsafe { apic::init_timer(6125) };

	unsafe {
		PICS.lock().write_masks(0b11111101, 0b11111111);
	}

	nullex::init();

	match allocator::init_heap(&mut mapper, &mut frame_allocator) {
		Ok(()) => println!("Heap initialized successfully"),
		Err(e) => panic!("Heap initialization failed: {:?}", e)
	}

	let mut fs = FileSystem::new();
	fs.create_file("/hello.txt", Permission::all()).unwrap();
	fs.write_file("/hello.txt", b"Hello Kernel World!").unwrap();
	fs.create_dir("/mydir", Permission::all()).unwrap();
	fs.create_file("/mydir/test.txt", Permission::all())
		.unwrap();
	fs.write_file("/mydir/test.txt", b"Secret message").unwrap();

	fs.create_dir("/logs", Permission::all()).unwrap();

	fs::init_fs(fs);

	//SYSLOG_SINK.log("Hello", LogLevel::Info);

	WRITER.lock().clear_everything();
	WRITER.lock().set_colors(Color16::White, Color16::Black);

	crate::keyboard::commands::init_commands();

	// Spawn the keyboard process.
	let _keyboard_pid = spawn_process(
		|_state| Box::pin(keyboard::print_keypresses()) as Pin<Box<dyn Future<Output = i32>>>,
		false
	);

	// Spawn process one.
	let _process1_pid = spawn_process(
		|state| Box::pin(process_one(state)) as Pin<Box<dyn Future<Output = i32>>>,
		false
	);

	// Spawn process two.
	let _process2_pid = spawn_process(
		|state| Box::pin(process_two(state)) as Pin<Box<dyn Future<Output = i32>>>,
		false
	);

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
