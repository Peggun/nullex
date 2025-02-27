// main.rs

/*
This file is the main entry point of the Nullex kernel. It defines the core logic and initialization procedures for the operating system.
*/

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(nullex::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use nullex::{allocator, apic::apic, fs::{self, ramfs::{FileSystem, Permission}}, interrupts::PICS, memory::{self, translate_addr, BootInfoFrameAllocator}, println, task::{executor::EXECUTOR, keyboard, Process}, vga_buffer::WRITER};

use vga::colors::Color16;

entry_point!(kernel_main);

#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use x86_64::VirtAddr;

    //print!("test@nullex: $ ");
    //WRITER.lock().input_start_column = WRITER.lock().column_position;

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map)
    };

    // Setup APIC Timer
    unsafe { apic::enable_apic() };
    memory::map_apic(&mut mapper, &mut frame_allocator);

    unsafe {
        PICS.lock().write_masks(0b11111101, 0b11111111);
    }

    nullex::init();

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



    // Spawn the keyboard process
    crate::keyboard::commands::init_commands();

    let keyboard_process = Process::new(
        EXECUTOR.lock().create_pid(), // Get a new PID from the executor
        keyboard::print_keypresses()
    );
    EXECUTOR.lock().spawn_process(keyboard_process);


    // Run the executor
    EXECUTOR.lock().run();
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