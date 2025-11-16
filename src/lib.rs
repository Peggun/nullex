// lib.rs

/*
Kernel module for the kernel.
*/

#![no_std]
#![allow(internal_features)]
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
#![feature(ptr_internals)]

#[macro_use]
extern crate alloc;
extern crate core;

pub mod allocator;
pub mod apic;
pub mod common;
pub mod config;
pub mod constants;
pub mod drivers;
pub mod error;
pub mod fs;
pub mod gdt;
pub mod interrupts;
pub mod io;
pub mod ioapic;
pub mod memory;
pub mod pit;
pub mod serial;
pub mod syscall;
pub mod task;
pub mod utils;
pub mod vga_buffer;

use alloc::boxed::Box;
use core::{
	future::Future,
	hint::spin_loop,
	pin::Pin,
	sync::atomic::Ordering,
	task::{Context, Poll}
};

use lazy_static::lazy_static;
use x86_64::VirtAddr;

use crate::{
	apic::APIC_BASE,
	common::ports::{inb, outb},
	constants::initialize_constants,
	fs::ramfs::{FileSystem, Permission},
	io::keyboard::line_editor::print_keypresses,
	memory::BootInfoFrameAllocator,
	task::{
		Process,
		executor::{self, CURRENT_PROCESS, EXECUTOR},
		keyboard
	},
	utils::{multiboot2::parse_multiboot2, mutex::SpinMutex, process::spawn_process},
	vga_buffer::WRITER
};

lazy_static! {
	pub static ref PHYS_MEM_OFFSET: SpinMutex<VirtAddr> = SpinMutex::new(VirtAddr::new(0x0));
}

pub fn raw_serial_test() {
	unsafe {
		for &b in b"HELLO\r\n" {
			// wait for Transmitter Holding Register Empty (LSR bit 5)
			while (inb(0x3F8 + 5) & 0x20) == 0 {
				spin_loop();
			}
			outb(0x3F8, b);
		}
	}
}

pub fn init() {
	serial_println!("[Info] Initializing kernel...");
	gdt::init();
	serial_println!("gdt done");
	interrupts::init_idt();
	serial_println!("[Info] Finished IDT Init...");
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

	fs::init_fs(fs);
}

#[repr(C)]
pub struct MultibootBootInfo {
	pub flags: usize,
	pub mem_lower: usize,
	pub mem_upper: usize
}

#[unsafe(no_mangle)]
pub extern "C" fn kernel_main(mbi_addr: usize) -> ! {
	WRITER.lock().clear_everything();
	println!("[Info] Starting Kernel Init...");

	let boot_info = unsafe { parse_multiboot2(mbi_addr) };

	let pmo = PHYS_MEM_OFFSET.lock();
	let mut mapper = unsafe { memory::init(*pmo) };
	let memory_map_static: &'static _ = unsafe { core::mem::transmute(&boot_info.memory_map) };
	let mut frame_allocator = BootInfoFrameAllocator::init(memory_map_static);

	// mask legacy PIC IRQs
	// always need to mask these
	// double fault if not masked
	unsafe {
		outb(0x21, 0xFF);
		outb(0xA1, 0xFF);
	}

	crate::init();

	match allocator::init_heap(&mut mapper, &mut frame_allocator) {
		Ok(()) => println!("Heap initialized successfully"),
		Err(e) => panic!("Heap initialization failed: {:?}", e)
	}

	// 1) set APIC_BASE to the virtual mapping base (physical offset + APIC phys)
	{
		let mut apic_base = APIC_BASE.lock();
		*apic_base = pmo.as_u64() as usize + 0xFEE0_0000usize;
	}

	// 2) map the local APIC MMIO into your virtual address space
	memory::map_apic(&mut mapper, &mut frame_allocator, *pmo);

	// 3) now it's safe to enable the local APIC (writes SVR)
	unsafe { apic::enable_apic() };

	// 4) initialize the APIC timer (reads/writes APIC registers)
	unsafe { apic::init_timer() };

	// 5) map the IOAPIC MMIO
	memory::map_ioapic(&mut mapper, &mut frame_allocator, *pmo);

	// 6) create IoApic using the virtual base and init it
	let ioapic_virt_base = (*pmo).as_u64() + 0xFEC0_0000u64;
	let mut ioapic = unsafe { ioapic::IoApic::new(ioapic_virt_base) };
	let lapic_id = unsafe { (apic::read_register(apic::ID) >> 24) as u8 };
	unsafe {
		ioapic.init(32, lapic_id);
	} // offset 32, dest = local apic id

	// LAPIC id

	unsafe { apic::init_timer() };
	initialize_constants();

	let fs = FileSystem::new();

	println!("[Info] Initializing RAMFS...");

	// setup files and ramfs.
	setup_system_files(fs);

	println!("[Info] Done.");

	//SYSLOG_SINK.log("Initialized Main Kernel Successfully\n", LogLevel::Info);

	WRITER.lock().clear_everything();
	// WRITER.lock().set_colors(Color16::White, Color16::Black);

	// Run init_commands in its own process so it doesn't run on the boot/kernel
	// stack.
	let _cmds_pid = spawn_process(
		|_state| {
			Box::pin(async move {
				crate::keyboard::commands::init_commands();
				0
			}) as Pin<Box<dyn Future<Output = i32>>>
		},
		false
	);
	// init_serial_input();
	// init_serial_commands();

	// Spawn the keyboard process.
	let _keyboard_pid = spawn_process(
		|_state| Box::pin(print_keypresses()) as Pin<Box<dyn Future<Output = i32>>>,
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
