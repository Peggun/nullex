//! lib.rs

//!
//! Kernel module for the kernel.
//!

#![no_std]
#![no_main]

#![allow(internal_features)]
#![warn(missing_docs)]

#![feature(abi_x86_interrupt)]
#![feature(step_trait)]
#![feature(associated_type_defaults)]
#![feature(alloc_error_handler)]
#![feature(str_from_raw_parts)]
#![feature(generic_atomic)]
#![feature(string_from_utf8_lossy_owned)]
#![feature(ptr_internals)]
#![feature(fn_traits)]
#![feature(macro_metavar_expr_concat)]
#![feature(new_range_api)]
#![feature(allocator_api)]

#[macro_use]
extern crate alloc;
extern crate core;

pub mod acpi;
pub mod allocator;
pub mod apic;
pub mod arch;
pub mod common;
pub mod config;
pub mod drivers;
pub mod error;
pub mod fs;
pub mod gdt;
pub mod gsi;
pub mod interrupts;
pub mod io;
#[allow(missing_docs)]
pub mod ioapic;
pub mod memory;
pub mod net;
pub mod pit;
pub mod rtc;
#[allow(deprecated)]
pub mod serial;
pub mod syscall;
pub mod task;
pub mod utils;
pub mod vga_buffer;

use alloc::boxed::Box;
use core::{
	future::Future,
	pin::Pin,
	sync::atomic::Ordering,
	task::{Context, Poll}
};

use x86_64::{
	VirtAddr,
	instructions::{hlt, interrupts::enable, port::Port}
};

use crate::{
	acpi::link_isos,
	allocator::ALLOCATOR_INFO,
	apic::{APIC_BASE, APIC_TPS},
	common::ports::outb,
	fs::ramfs::{FileSystem, setup_system_files},
	interrupts::APIC_TIMER_VECTOR,
	io::{
		keyboard::line_editor::print_keypresses,
		pci::{self, discover_pci_devices}
	},
	ioapic::{IOAPIC, dump_gsi},
	memory::{BootInfoFrameAllocator, init_global_alloc},
	task::{
		Process,
		executor::{self, CURRENT_PROCESS, EXECUTOR},
		keyboard
	},
	utils::{multiboot2::parse_multiboot2, mutex::SpinMutex, process::spawn_process}
};
// Bring in virtio driver registration function explicitly so we can register drivers
use crate::drivers::virtio::net::virtio_net_driver_init;

lazy_static! {
	/// Static reference to the physical memory offset for the kernel.
	pub static ref PHYS_MEM_OFFSET: SpinMutex<VirtAddr> = SpinMutex::new(VirtAddr::new(0x0));
}

fn init() {
	serial_println!("[Info] Initializing kernel...");
	gdt::init();
	serial_println!("[Info] GDT done.");
	unsafe { interrupts::init_idt() };
	serial_println!("[Info] Finished IDT Init.");
	serial_println!("[Info] Done.");
}

fn hlt_loop() -> ! {
	loop {
		x86_64::instructions::hlt();
	}
}

#[unsafe(no_mangle)]
/// Entry point for the kernel.
pub unsafe extern "C" fn kernel_main(mbi_addr: usize) -> ! {
	clear_screen!();
	println!("[Info] Starting Kernel Init...");

	// Parse boot info and initialize memory
	let boot_info = unsafe { parse_multiboot2(mbi_addr) };
	let pmo_val = *PHYS_MEM_OFFSET.lock();
	let mapper = unsafe { memory::init(pmo_val) };
	let memory_map_static: &'static _ = unsafe { core::mem::transmute(&boot_info.memory_map) };
	let frame_allocator = BootInfoFrameAllocator::init(memory_map_static);

	if let Err(e) = init_global_alloc(mapper, frame_allocator) {
		panic!("Global Allocator Initialization failed: {}", e);
	}

	// Initialize GDT and IDT
	crate::init();

	// Setup APIC and IOAPIC
	{
		let mut m_lock = ALLOCATOR_INFO.mapper.lock();
		let mut f_lock = ALLOCATOR_INFO.frame_allocator.lock();
		let mapper = m_lock.as_mut().unwrap();
		let frame_allocator = f_lock.as_mut().unwrap();

		*APIC_BASE.lock() = pmo_val.as_u64() as usize + 0xFEE0_0000usize;
		memory::map_apic(*mapper, *frame_allocator, pmo_val);
		memory::map_ioapic(*mapper, *frame_allocator, pmo_val);
	}

	unsafe {
		apic::enable_apic(0xFF);
	}

	rtc::init_rtc();
	match apic::calibrate(1024) {
		Ok((ticks_per_sec, initial_count)) => {
			serial_println!("APIC ticks/sec = {}", ticks_per_sec);
			APIC_TPS.store(ticks_per_sec, Ordering::SeqCst);
			unsafe {
				apic::mask_timer(true);
				apic::start_timer_periodic(APIC_TIMER_VECTOR, initial_count);
				apic::mask_timer(false);
			}
		}
		Err(e) => serial_println!("APIC calibration failed: {}", e)
	}

	let mut ioapic = IOAPIC.lock();
	let lapic_id = unsafe { (apic::read_register(apic::APIC_ID) >> 24) as u8 };
	unsafe { ioapic.init(32, lapic_id) };
	drop(ioapic);

	// Mask legacy PIC
	unsafe {
		outb(0x21, 0xFF);
		outb(0xA1, 0xFF);
	}

	serial_println!("[ACPI] ACPI tables parsed (RSDT available)");

	// Setup filesystem
	println!("[Info] Initializing RAMFS and preparing PCI...");
	let fs = FileSystem::new();
	setup_system_files(fs);

	serial_println!("[PCI] Registering platform drivers before PCI discovery...");
	virtio_net_driver_init();

	discover_pci_devices();

	// Link ISOs and program IOAPIC
	unsafe {
		link_isos();
	}

	serial_println!("[INIT] Finalizing all PCI devices...");
	if let Err(e) = pci::finalize_all_devices() {
		panic!("Failed to finalize PCI devices: {}", e);
	}

	serial_println!("[INIT] Enabling CPU interrupts...");
	enable();
	serial_println!("[INIT] Interrupts enabled successfully!");

	dump_gsi(11);

	// network init
	crate::net::init();
	serial_println!("[NET] Resolving gateway MAC...");
	let _ = crate::net::send_arp_request(crate::net::GATEWAY_IP);

	match crate::net::arp::wait_for_arp(crate::net::GATEWAY_IP, 2000) {
		Ok(mac) => {
			serial_println!(
				"[NET] Gateway MAC: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
				mac[0],
				mac[1],
				mac[2],
				mac[3],
				mac[4],
				mac[5]
			);
		}
		Err(e) => {
			serial_println!("[NET] Could not resolve gateway: {}", e);
		}
	}

	// Give it time to resolve
	for _ in 0..10000 {
		core::hint::spin_loop();
	}

	// Check if it resolved
	if let Some(mac) = crate::net::arp::ARP_CACHE
		.lock()
		.iter()
		.find(|(ip, _)| *ip == crate::net::GATEWAY_IP)
		.map(|(_, mac)| *mac)
	{
		serial_println!(
			"[NET] Gateway MAC: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
			mac[0],
			mac[1],
			mac[2],
			mac[3],
			mac[4],
			mac[5]
		);
	} else {
		serial_println!("[NET] WARNING: Gateway MAC not resolved!");
	}

	WRITER.lock().clear_everything();

	// Spawn processes
	let _cmds_pid = spawn_process(
		|_state| {
			Box::pin(async move {
				crate::keyboard::commands::init_commands();
				0
			}) as Pin<Box<dyn Future<Output = i32>>>
		},
		false
	);

	let _keyboard_pid = spawn_process(
		|_state| Box::pin(print_keypresses()) as Pin<Box<dyn Future<Output = i32>>>,
		false
	);

	// Main executor loop
	let process_queue = EXECUTOR.lock().process_queue.clone();
	loop {
		if let Some(pid) = process_queue.pop() {
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
				*CURRENT_PROCESS.lock() = Some(process_arc.lock().state.clone());

				let mut process = process_arc.lock();
				let process_state = process.state.clone();
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
				*CURRENT_PROCESS.lock() = None;
			}
		} else {
			EXECUTOR.lock().sleep_if_idle();
		}
	}
}

#[allow(unused)]
fn qemu_exit(code: u32) -> ! {
	serial_println!("QEMU exit: guest code = {}", code);

	let mut port = Port::<u32>::new(0xf4);
	unsafe {
		port.write(code);
	}

	loop {
		hlt();
	}
}

/// This function is called on panic.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
	println!("{}", info);
	crate::hlt_loop();
}