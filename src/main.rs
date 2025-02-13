// main.rs
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nullex::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use nullex::{allocator, memory::{self, BootInfoFrameAllocator}, print, println, task::{executor::Executor, keyboard, Task}};

entry_point!(kernel_main);

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}

#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use x86_64::VirtAddr;

    print!("test@nullex: $ ");
    //WRITER.lock().input_start_column = WRITER.lock().column_position;

    nullex::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    let mut executor = Executor::new(); // new
    //executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    nullex::hlt_loop();
}