// main.rs
#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nullex::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use nullex::{align_buffer, allocator, fs::{self, ata::AtaDisk, ramfs::{FileSystem, Permission}}, hlt_loop, memory::{self, translate_addr, BootInfoFrameAllocator}, println, task::{executor::Executor, keyboard, Task}, vga_buffer::WRITER};
use x86_64::VirtAddr;
use zerocopy::FromBytes;

use vga::colors::{Color16, TextModeColor};
use vga::writers::{ScreenCharacter, TextWriter, Text80x25};

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

    match allocator::init_heap(&mut mapper, &mut frame_allocator) {
        Ok(()) => println!("Heap initialized successfully"),
        Err(e) => panic!("Heap initialization failed: {:?}", e),
    }

    let test_addr = VirtAddr::new(0x4444_4444_0000);
    let phys_addr = unsafe { translate_addr(test_addr, phys_mem_offset) }
        .expect("Failed to translate heap address");
    //println!("Heap phys addr: {:?}", phys_addr);

    let mut fs = FileSystem::new();

    fs.create_file("/hello.txt", Permission::all()).unwrap();
    fs.write_file("/hello.txt", b"Hello Kernel World!").unwrap();
    
    fs.create_dir("/mydir", Permission::all()).unwrap();
    fs.create_file("/mydir/test.txt", Permission::all()).unwrap();
    fs.write_file("/mydir/test.txt", b"Secret message").unwrap();

    fs::init_fs(fs);

    WRITER.lock().clear_everything();
    WRITER.lock().set_colors(Color16::White, Color16::Black);

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

#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    nullex::hlt_loop();
}