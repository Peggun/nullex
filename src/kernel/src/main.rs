// main.rs

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use core::{
	arch::asm,
	sync::atomic::Ordering,
	task::{Context, Poll}
};

use bootloader::{BootInfo, entry_point};
use kernel::{
	allocator, apic::{self, apic::sleep}, constants::{initialize_constants, SYSLOG_SINK}, errors::KernelError, fs::{
		self,
		ata::AtaDisk,
		ramfs::{FileSystem, Permission}
	}, hlt_loop, interrupts::PICS, memory::{self, BootInfoFrameAllocator}, println, serial, serial_println, syscall::invoke_syscall, task::{
		executor::{self, allocate_kernel_stack, deallocate_kernel_stack, run_combined_executor, run_executor, ProcessWaker, UserProcess, UserProcessState, CURRENT_PROCESS, EXECUTOR},
		keyboard, Process, ProcessId, ProcessState
	}, utils::{
		logger::{levels::LogLevel, traits::logger_sink::LoggerSink},
		process::spawn_process
	}, vga_buffer::WRITER
};
use lazy_static::lazy_static;
use orchestrator::syscall_interface::{SYS_EXIT, SYS_PRINT};
use raw_cpuid::CpuId;
use vga::colors::Color16;
use x86_64::{
	VirtAddr,
	instructions::tlb::flush,
	structures::paging::{FrameAllocator, Mapper, Page, PageTable, PageTableFlags}
};
use kernel::task::yield_now; // Import yield_now from your task module
use kernel::task::executor::{CURRENT_PID, PROCESS_QUEUE};

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
}

/// Process 2: prints a message every half second.
async fn process_two(_state: Arc<ProcessState>) -> Result<i32, KernelError> {
	loop {
		serial_println!("Process 2: Hello every half second");
		sleep_half_second().await;
	}
}

/// Idle process: never terminates, simply yields control.
async fn idle_process(_state: Arc<ProcessState>) -> Result<i32, KernelError> {
	loop {
		// Yield to the scheduler and then continue looping
		yield_now().await;
	}
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

	serial_println!("[Debug] Physical memory offset: {:#x}", phys_mem_offset.as_u64());

	kernel::init();

	match allocator::init_heap(&mut mapper, &mut frame_allocator) {
		Ok(()) => println!("Heap initialized successfully"),
		Err(e) => panic!("Heap initialization failed: {:?}", e)
	}

	unsafe {
		let _ = apic::apic::enable_apic();
	};
	memory::map_apic(&mut mapper, &mut frame_allocator);
	
	unsafe {
		let _ = apic::apic::init_timer(1_000_000).expect("Failed to initialize APIC Timer");
	};

	unsafe { serial_println!("[Debug] Timer IST stack: {:#x}", &kernel::gdt::TSS.interrupt_stack_table[2].as_u64()); }
	unsafe {
		let stack_top = allocate_kernel_stack();
		let ptr = (stack_top - 8) as *mut u64;
		*ptr = 0xDEADBEEF;
		let bool = *ptr == 0xDEADBEEF;
		serial_println!("[Debug] Kernel stack allocated at {:#x}, value: {:#x}, success: {}", stack_top, *ptr, bool);
		deallocate_kernel_stack(stack_top);
	}

	// unsafe {
	// 	let stack_ptr = kernel::gdt::TSS.interrupt_stack_table[2].as_mut_ptr::<u64>();
	// 	*stack_ptr = 0; // Try writing to it
	// }

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

	let mut sector_buffer = [0u8; 512];
	let mut ata = unsafe { AtaDisk::new() };
	unsafe {
		ata.read_disk_sector(0, &mut sector_buffer)
			.expect("Failed to read sector");
	}

	/// Set up the test program
	let user_prog_frame = frame_allocator
		.allocate_frame()
		.expect("Failed to allocate frame for user program");

	// Allocate a second frame for the page after the test program
	let user_prog_frame_next = frame_allocator
		.allocate_frame()
		.expect("Failed to allocate frame for user program extra page");

	let user_prog_virt_addr = phys_mem_offset + user_prog_frame.start_address().as_u64();
	let user_prog_ptr = user_prog_virt_addr.as_mut_ptr::<u8>();

	// Your test program, loaded at 0x100000.
	let test_program: [u8; 2] = [
		0xEB, 0xFE, // jmp $
	];

	// Copy the test program into the first page.
	unsafe {
		for i in 0..test_program.len() {
			*user_prog_ptr.add(i) = test_program[i];
		}
		asm!("mfence");
	}

	// Map the first page for the user program at 0x100000.
	let user_prog_page = Page::containing_address(VirtAddr::new(0x100000));
	let user_prog_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
	unsafe {
	mapper
		.map_to(
			user_prog_page,
			user_prog_frame,
			user_prog_flags,
			&mut frame_allocator
		)
		.expect("Failed to map user program")
		.flush();
	flush(VirtAddr::new(0x100000));
	}

	// Map an extra page right after the first (at 0x101000) so that any return address landing
	// at the end of the first page is covered.
	let user_prog_page_next = Page::containing_address(VirtAddr::new(0x101000));
	unsafe {
	mapper
		.map_to(
			user_prog_page_next,
			user_prog_frame_next,
			user_prog_flags,
			&mut frame_allocator
		)
		.expect("Failed to map extra user program page")
		.flush();
	}
	
	// Allocate and map user stack
	let stack_frame = frame_allocator
		.allocate_frame()
		.expect("Failed to allocate frame for user stack");
	let stack_page = Page::containing_address(VirtAddr::new(0x300000));
	let stack_flags =
		PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE;
	unsafe {
		mapper
			.map_to(stack_page, stack_frame, stack_flags, &mut frame_allocator)
			.expect("Failed to map user stack")
			.flush();
	}

	let stack_frame2 = frame_allocator
		.allocate_frame()
		.expect("Failed to allocate second frame for user stack");
	let stack_page2 = Page::containing_address(VirtAddr::new(0x301000));
	unsafe {
		mapper
			.map_to(stack_page2, stack_frame2, stack_flags, &mut frame_allocator)
			.expect("Failed to map second user stack page")
			.flush();
	}

	let stack_frame3 = frame_allocator
		.allocate_frame()
		.expect("Failed to allocate second frame for user stack");
	let stack_page3 = Page::containing_address(VirtAddr::new(0x302000));
	unsafe {
		mapper
			.map_to(stack_page3, stack_frame3, stack_flags, &mut frame_allocator)
			.expect("Failed to map second user stack page")
			.flush();
	}

	// Initialize the process queue
	unsafe {
		kernel::task::executor::PROCESS_QUEUE.lock().replace(Vec::new());
		if let Some(queue) = kernel::task::executor::PROCESS_QUEUE.lock().as_mut() {
			// Allocate kernel stack for the process
			let kernel_stack_top = kernel::task::executor::allocate_kernel_stack();
			
			// Push the user process into the queue
			queue.push(kernel::task::executor::UserProcess {
				id: ProcessId::new(1),
				entry_point: 0x100000,      // Matches your test program address
				stack_pointer: 0x301ff0,    // Matches your user stack mapping
				kernel_stack_top,           // Newly allocated kernel stack
				state: kernel::task::executor::UserProcessState::Ready,
			});
			println!("[Info] User process with ID 1 added to PROCESS_QUEUE");
		}
	}

	println!("[Info] Done.");
	SYSLOG_SINK.log("Initialized Main Kernel Successfully\n", LogLevel::Info);

	WRITER.lock().clear_everything();
	WRITER.lock().set_colors(Color16::White, Color16::Black);

	let _ = crate::keyboard::commands::init_commands();

	// Spawn the four kernel processes
	let _keyboard_pid = spawn_process(|_state| Box::pin(keyboard::print_keypresses()), false);
	let _process1_pid = spawn_process(|state| Box::pin(process_one(state)), false);
	let _process2_pid = spawn_process(|state| Box::pin(process_two(state)), false);
	let _idle_pid = spawn_process(|state| Box::pin(idle_process(state)), false);

	//serial_println!("test");

	serial_println!("[Info] Starting combined executor...");
	run_executor();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
	println!("{}", info);
	kernel::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
	println!("{}", info);
	kernel::hlt_loop();
}
