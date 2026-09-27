//!
//! lib.rs
//!
//! Kernel module for the kernel.

#![no_std]
#![no_main]
#![allow(internal_features)]
#![warn(missing_docs)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(str_from_raw_parts)]
#![feature(ptr_internals)]
#![feature(new_range_api)]
#![feature(pointer_is_aligned_to)]

#[macro_use]
extern crate alloc;
extern crate core;
pub mod acpi;
pub mod allocator;
pub mod apic;
pub mod arch;
pub mod boot;
pub mod common;
pub mod config;
pub mod crypto;
pub mod debug;
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
pub mod process;
pub mod rtc;
#[allow(deprecated)]
pub mod serial;
pub mod sync;
pub mod syscall;
pub mod task;
pub mod testing;
pub mod time;
pub mod vga_buffer;

const _: () = assert!(cfg!(getrandom_backend = "custom"));

use alloc::{boxed::Box, vec::Vec};
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
	boot::{enable_sse, init_efer, multiboot2::parse_multiboot2},
	common::ports::outb,
	crypto::rng::kernel_entropy_fill,
	debug::logger::init_logging,
	drivers::virtio::net::virtio_net_driver_init,
	fs::ramfs::{FileSystem, setup_system_files},
	interrupts::APIC_TIMER_VECTOR,
	io::{
		keyboard::line_editor::print_keypresses,
		pci::{self, discover_pci_devices}
	},
	ioapic::{IOAPIC, dump_gsi},
	memory::{BootInfoFrameAllocator, MMIO_BASE, PHYS_MEM_OFFSET, init_global_alloc},
	process::util::{spawn_process, spawn_user_process},
	task::{
		executor::{self, CURRENT_PROCESS, EXECUTOR},
		process::{Process, ProcessId}
	}
};

// lazy_static! {
// 	/// Static reference to the physical memory offset for the kernel.
// 	pub static ref PHYS_MEM_OFFSET: SpinMutex<VirtAddr> =
// SpinMutex::new(VirtAddr::new(0x0)); }

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

	init_efer();
	enable_sse();
	init_logging();

	// parse boot info and initialize memory
	let boot_info = unsafe { parse_multiboot2(mbi_addr) };
	let mapper = unsafe { memory::init(VirtAddr::new(PHYS_MEM_OFFSET)) };
	let memory_map_static: &'static _ = unsafe { core::mem::transmute(&boot_info.memory_map) };
	let frame_allocator = BootInfoFrameAllocator::init(memory_map_static);

	if let Err(e) = init_global_alloc(mapper, frame_allocator) {
		panic!("Global Allocator Initialization failed: {}", e);
	}

	// todo: actually use the physical memory offset. i will add that very very
	// soon.

	// init gdt and idt
	crate::init();

	// setup apic and ioapic
	{
		let mut m_lock = ALLOCATOR_INFO.mapper.lock();
		let mut f_lock = ALLOCATOR_INFO.frame_allocator.lock();
		let mapper = m_lock
			.as_mut()
			.expect("FATAL: Mapper not initialized during APIC setup");
		let frame_allocator = f_lock
			.as_mut()
			.expect("FATAL: Frame allocator not initialized during APIC setup");

		*APIC_BASE.lock() = MMIO_BASE as usize;
		memory::map_apic(*mapper, *frame_allocator);
		memory::map_ioapic(*mapper, *frame_allocator);
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
	// i will need to fix this, and make like a dynamically found way to load
	// the virtio-net driver
	virtio_net_driver_init();

	discover_pci_devices();

	// Link ISOs and program IOAPIC
	unsafe {
		link_isos();
	}

	let mut ioapic = IOAPIC.lock();
	let lapic_id = unsafe { (apic::read_register(apic::APIC_ID) >> 24) as u8 };
	unsafe { ioapic.init(32, lapic_id) };
	unsafe { ioapic.enable_irq(8) };
	drop(ioapic);

	serial_println!("[INIT] Finalizing all PCI devices...");
	if let Err(e) = pci::finalize_all_devices() {
		panic!("Failed to finalize PCI devices: {}", e);
	}

	serial_println!("[INIT] Enabling CPU interrupts...");
	enable();
	serial_println!("[INIT] Interrupts enabled successfully!");

	dump_gsi(8);

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

	#[cfg(feature = "test")]
	{
		use crate::testing::ktest::{run_all_tests, run_user_tests};

		run_all_tests();
		let ktest_pid = run_user_tests();
		crate::task::executor::run_executor(Some(ktest_pid));
	}

	#[cfg(not(feature = "test"))]
	{
		let _keyboard_pid = match spawn_process(
			|_state| Box::pin(print_keypresses()) as Pin<Box<dyn Future<Output = i32>>>,
			false
		) {
			Ok(pid) => pid,
			Err(e) => {
				serial_println!("[ERROR] Failed to spawn keyboard process: {}", e);
				ProcessId::new(0)
			}
		};

		let _nush_pid = fs::with_fs(|fs| match fs.read_file("/init/nush.elf") {
			Ok(bytes) => match spawn_user_process(bytes, Vec::new(), Vec::new()) {
				Ok(proc) => {
					let pid = proc.state.id;
					{
						let mut executor = EXECUTOR.lock();
						if let Err(e) = executor.spawn_process(proc) {
							serial_println!("[ERROR] Failed to queue NUSH process: {}", e);
							return ProcessId::new(0);
						}
					}
					pid
				}
				Err(e) => {
					serial_println!("[ERROR] Failed to spawn NUSH process: {}", e);
					ProcessId::new(0)
				}
			},
			Err(_) => {
				serial_println!("[ERROR] NUSH is missing!");
				ProcessId::new(0)
			}
		});

		crate::task::executor::run_executor(None);
	}
}

/// Exits QEMU with the given exit code.
#[allow(unused)]
pub fn qemu_exit(code: u32) -> ! {
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

/// This function is called by the guest to get random bytes used for
/// cryptographic operations.
#[unsafe(no_mangle)]
pub unsafe extern "Rust" fn __getrandom_v03_custom(
	dest: *mut u8,
	len: usize
) -> Result<(), getrandom::Error> {
	if len == 0 {
		return Ok(());
	}

	let buf = unsafe { core::slice::from_raw_parts_mut(dest, len) };
	kernel_entropy_fill(buf)
}
