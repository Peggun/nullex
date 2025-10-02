#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nullex::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

use bootloader::{BootInfo, entry_point};
use nullex::{allocator::HEAP_SIZE, apic, println};

entry_point!(main);

use alloc::{boxed::Box, vec::Vec};

#[test_case]
fn many_boxes() {
	for i in 0..HEAP_SIZE {
		let x = Box::new(i);
		assert_eq!(*x, i);
	}
}

#[test_case]
fn many_boxes_long_lived() {
	let long_lived = Box::new(1); // new
	for i in 0..HEAP_SIZE {
		let x = Box::new(i);
		assert_eq!(*x, i);
	}
	assert_eq!(*long_lived, 1); // new
}

#[test_case]
fn simple_allocation() {
	let heap_value_1 = Box::new(41);
	let heap_value_2 = Box::new(13);
	assert_eq!(*heap_value_1, 41);
	assert_eq!(*heap_value_2, 13);
}

#[test_case]
fn large_vec() {
	let n = 1000;
	let mut vec = Vec::new();
	for i in 0..n {
		vec.push(i);
	}
	assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

fn main(boot_info: &'static BootInfo) -> ! {
	use nullex::{
		allocator,
		memory::{self, BootInfoFrameAllocator}
	};
	use x86_64::VirtAddr;

	let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
	let mut mapper = unsafe { memory::init(phys_mem_offset) };
	let mut frame_allocator = BootInfoFrameAllocator::init(&boot_info.memory_map);

	// Setup APIC Timer
	unsafe { apic::enable_apic() };
	memory::map_apic(&mut mapper, &mut frame_allocator);

	nullex::init();

	match allocator::init_heap(&mut mapper, &mut frame_allocator) {
		Ok(()) => println!("Heap initialized successfully"),
		Err(e) => panic!("Heap initialization failed: {:?}", e)
	}

	test_main();
	loop {
		x86_64::instructions::hlt();
	}
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
	nullex::test_panic_handler(info)
}
