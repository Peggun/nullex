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

use core::{panic::PanicInfo, task::Poll, future::Future};

use alloc::boxed::Box;
use bootloader::{entry_point, BootInfo};
use nullex::{
    allocator,
    apic::apic,
    fs::{
        self,
        ramfs::{FileSystem, Permission},
    },
    interrupts::PICS,
    memory::{self, translate_addr, BootInfoFrameAllocator},
    println,
    task::{executor::{ProcessWaker, EXECUTOR}, keyboard, ForeverPending, Process},
    vga_buffer::WRITER,
};

use vga::colors::Color16;
use core::task::Context;

entry_point!(kernel_main);

#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use x86_64::VirtAddr;

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

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
        EXECUTOR.lock().create_pid(),
        keyboard::print_keypresses(),
    );
    EXECUTOR.lock().spawn_process(keyboard_process);

    // Get a clone of the process_queue for concurrent access
    let process_queue = EXECUTOR.lock().process_queue.clone();

    //for i in 0..3 {
        //let pid = EXECUTOR.lock().create_pid(); // Generate a unique ProcessId
        //let future = ForeverPending;           // Create the forever-pending future
        //let process = Process::new(pid, Box::pin(future)); // Wrap it in a pinned box
        //EXECUTOR.lock().spawn_process(process); // Spawn the process
        //println!("Spawned dummy process {}", pid.get()); // Optional: confirm spawning
    //}
    
    // Main executor loop
    loop {
        if let Some(pid) = process_queue.pop() {
            // Briefly lock EXECUTOR to get the process
            let process_arc = {
                let executor = EXECUTOR.lock();
                executor.processes.get(&pid).cloned()
            };
            if let Some(process_arc) = process_arc {
                // Lock the individual process and poll it
                let mut process = process_arc.lock();
                let waker = {
                    let mut executor = EXECUTOR.lock();
                    executor.waker_cache
                        .entry(pid)
                        .or_insert_with(|| ProcessWaker::new(pid, process_queue.clone()))
                        .clone()
                };
                let mut context = Context::from_waker(&waker);
                let result = process.poll(&mut context);
                if let Poll::Ready(exit_code) = result {
                    // Lock EXECUTOR again to remove the finished process
                    let mut executor = EXECUTOR.lock();
                    executor.processes.remove(&pid);
                    executor.waker_cache.remove(&pid);
                    println!("Process {} exited with code: {}", pid.get(), exit_code);
                }
            }
        } else {
            EXECUTOR.lock().sleep_if_idle();
        }
    }
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
