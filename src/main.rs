// main.rs
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nullex::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use nullex::{align_buffer, allocator, fs::{ata::AtaDisk, ext2::superblock::Ext2Superblock}, hlt_loop, memory::{self, translate_addr, BootInfoFrameAllocator}, println, task::{executor::Executor, keyboard, Task}, vga_buffer::clear_screen};
use x86_64::VirtAddr;
use zerocopy::FromBytes;

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

    //print!("test@nullex: $ ");
    //WRITER.lock().input_start_column = WRITER.lock().column_position;

    nullex::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");

    read_ext2(boot_info);

    clear_screen();

    let mut executor = Executor::new(); // new
    //executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

fn read_ext2(boot_info: &'static BootInfo) {
    // 3. Force stack-allocated buffer alignment
    let mut sector = align_buffer([0u8; 512]);
    
    // 4. Verify LBA calculation (ext2 superblock is at LBA 2 for 512b sectors)
    let lba = 2;
    
    // 5. Print full diagnostic info
    println!(
        "Buffer: virt={:?} phys={:?}",
        VirtAddr::from_ptr(sector.inner().as_ptr()),
        unsafe { translate_addr(VirtAddr::from_ptr(sector.inner().as_ptr()), VirtAddr::new(boot_info.physical_memory_offset)) }
    );
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    nullex::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    nullex::hlt_loop();
}