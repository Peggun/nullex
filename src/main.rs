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

use core::{future::Future, panic::PanicInfo, pin::Pin, task::Poll};

use alloc::{boxed::Box, sync::Arc};
use bootloader::{entry_point, BootInfo};
use nullex::{
    allocator, apic::apic, fs::{
        self,
        ramfs::{FileSystem, Permission},
    }, interrupts::PICS, memory::{self, translate_addr, BootInfoFrameAllocator}, println, serial_println, syscall::{self, syscall}, task::{executor::{self, ProcessWaker, CURRENT_PROCESS, EXECUTOR}, keyboard, ForeverPending, Process, ProcessState}, utils::process::spawn_process, vga_buffer::WRITER
};

use vga::colors::Color16;
use core::task::Context;

entry_point!(kernel_main);

// Define a test process that uses sys_fork
async fn test_process(state: Arc<ProcessState>) -> i32 {
    if state.is_child {
        serial_println!("Child process {} started", state.id.get());
        ForeverPending.await
    } else {
        serial_println!("Parent process {} before fork", state.id.get());
        let child_pid = syscall::sys_fork();
        serial_println!("Parent process after fork, child PID: {}", child_pid);
        1 // Parent exit code
    }
}

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

    crate::keyboard::commands::init_commands();

    // Spawn the keyboard process
    let _keyboard_pid = spawn_process(|_state| {
        Box::pin(keyboard::print_keypresses()) as Pin<Box<dyn Future<Output = i32>>>
    }, false);

    // Spawn the test process
    let _test_pid = spawn_process(|_state| {
        Box::pin(test_process(_state)) as Pin<Box<dyn Future<Output = i32>>>
    }, false);

    // Main executor loop with CURRENT_PROCESS management
    let process_queue = EXECUTOR.lock().process_queue.clone();
    loop {
        if let Some(pid) = process_queue.pop() {
            let process_arc = {
                let executor = EXECUTOR.lock();
                executor.processes.get(&pid).cloned()
            };
            if let Some(process_arc) = process_arc {
                // Set the current process
                *CURRENT_PROCESS.lock() = Some(process_arc.lock().state.clone());
                
                let mut process = process_arc.lock();
                let waker = {
                    let mut executor = EXECUTOR.lock();
                    executor.waker_cache
                        .entry(pid)
                        .or_insert_with(|| executor::ProcessWaker::new(pid, process_queue.clone()))
                        .clone()
                };
                let mut context = Context::from_waker(&waker);
                let result = process.poll(&mut context);
                if let Poll::Ready(exit_code) = result {
                    let mut executor = EXECUTOR.lock();
                    executor.processes.remove(&pid);
                    executor.waker_cache.remove(&pid);
                    serial_println!("Process {} exited with code: {}", pid.get(), exit_code);
                }
                
                // Clear the current process
                *CURRENT_PROCESS.lock() = None;
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