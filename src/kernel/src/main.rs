// main.rs

/*
Main entry code for the kernel.

Note to self: you can use this to use the error handling
If needed:

let (Ok(my_var)|Err(my_var)) = foo(5);

https://stackoverflow.com/questions/76196072/get-value-t-from-resultt-t
*/

#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use kernel::hlt_loop;
use lazy_static::lazy_static;
use x86_64::instructions::tlb::flush;
use x86_64::structures::paging::{Mapper, Page, PageTable, PageTableFlags};
use x86_64::structures::paging::FrameAllocator;
use core::{
    arch::asm,
    sync::atomic::Ordering,
    task::{Context, Poll}
};

use bootloader::{BootInfo, entry_point};
use kernel::{
    allocator,
    apic::{self, apic::sleep},
    constants::{SYSLOG_SINK, initialize_constants},
    errors::KernelError,
    fs::{
        self,
        ata::AtaDisk,
        ramfs::{FileSystem, Permission}
    },
    interrupts::PICS,
    memory::{self, BootInfoFrameAllocator},
    println,
    serial_println,
    syscall::invoke_syscall,
    task::{
        Process,
        ProcessId,
        ProcessState,
        executor::{self, CURRENT_PROCESS, EXECUTOR},
        keyboard
    },
    utils::{
        logger::{levels::LogLevel, traits::logger_sink::LoggerSink},
        process::spawn_process
    },
    vga_buffer::WRITER
};
use orchestrator::syscall_interface::{SYS_EXIT, SYS_PRINT};
use raw_cpuid::CpuId;
use vga::colors::Color16;
use x86_64::VirtAddr;

entry_point!(kernel_main);

/// A dummy async delay approximating half a second.
async fn sleep_half_second() {
    unsafe {
        let _ = sleep(500).await;
    }
}

/// Process 1: prints a message every half second.
async fn process_one(_state: Arc<ProcessState>) -> Result<i32, KernelError> {
    loop {
        serial_println!("Process 1: Hello every half second");
        sleep_half_second().await;
    }
}

/// Process 2: prints a message every half second.
async fn process_two(_state: Arc<ProcessState>) -> Result<i32, KernelError> {
    loop {
        serial_println!("Process 2: Hello every half second");
        sleep_half_second().await;
    }
}

// Simple process structure for user-space processes
struct UserProcess {
    id: ProcessId,
    entry_point: usize,     // Virtual address of the program
    stack_pointer: usize,   // Top of the stack
    state: UserProcessState,
}

#[derive(PartialEq)]
enum UserProcessState {
    Ready,
    Running,
    Terminated,
}

// Global process queue
use spin::Mutex;

static PROCESS_QUEUE: Mutex<Option<Vec<UserProcess>>> = Mutex::new(None);

// Current process ID (for simplicity, not thread-safe yet)
static mut CURRENT_PID: Option<ProcessId> = None;

/// Switch to a user-space process
unsafe fn switch_to_process(process: &UserProcess) {
	unsafe {
		asm!(
			"mov ax, {data_sel}",
			"mov ds, ax",
			"mov es, ax",
			"mov fs, ax",
			"mov gs, ax",
			"push {data_sel_imm}",  // SS
			"push {user_stack}",    // RSP
			"pushfq",               // RFLAGS
			"pop rax",
			"or rax, 0x200",        // Enable interrupts
			"push rax",
			"push {code_sel}",      // CS
			"push {user_entry}",    // RIP
			"iretq",
			data_sel = const 0x1B_u16,
			data_sel_imm = const 0x1B_u64,
			user_stack = in(reg) process.stack_pointer,
			code_sel = const 0x13_u16,
			user_entry = in(reg) process.entry_point,
		);
	}
}

/// Executor loop to run processes
fn run_executor() -> ! {
    loop {
        unsafe {
            if let Some(queue) = PROCESS_QUEUE.lock().as_mut() {
                if let Some(process) = queue.iter_mut().find(|p| p.state == UserProcessState::Ready) {
                    process.state = UserProcessState::Running;
                    CURRENT_PID = Some(process.id);
                    serial_println!("[Info] Switching to process {}", process.id.get());
                    switch_to_process(process);
                    // Execution returns here after an interrupt (e.g., syscall)
                } else {
                    serial_println!("[Info] No ready processes, idling...");
                    asm!("hlt");
                }
            } else {
                panic!("Process queue not initialized");
            }
        }
    }
}

#[unsafe(no_mangle)]
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use x86_64::VirtAddr;

    println!("[Info] Starting Kernel Init...");

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    unsafe {
        PICS.lock().write_masks(0b11111101, 0b11111111);
    }

    kernel::init();

    match allocator::init_heap(&mut mapper, &mut frame_allocator) {
        Ok(()) => println!("Heap initialized successfully"),
        Err(e) => panic!("Heap initialization failed: {:?}", e)
    }

    unsafe {
        let _ = apic::apic::enable_apic();
    };
    memory::map_apic(&mut mapper, &mut frame_allocator);
    unsafe {
        let _ = apic::apic::init_timer(6125);
    };
    initialize_constants();

    let cpuid = CpuId::new();

    if let Some(vf) = cpuid.get_vendor_info() {
        serial_println!("Vendor Info: {}", vf.as_str())
    }

    let has_sse = cpuid
        .get_feature_info()
        .map_or(false, |finfo| finfo.has_sse());
    if has_sse {
        serial_println!("CPU supports SSE!");
    }

    if let Some(cparams) = cpuid.get_cache_parameters() {
        for cache in cparams {
            let size = cache.associativity()
                * cache.physical_line_partitions()
                * cache.coherency_line_size()
                * cache.sets();
            serial_println!("L{}-Cache size is {}", cache.level(), size);
        }
    } else {
        serial_println!("No cache parameter information available")
    }

    let mut fs = FileSystem::new();
    println!("[Info] Initializing RAMFS...");
    if let Err(e) = fs.create_dir("/logs", Permission::all()) {
        panic!("Failed to create /logs directory: {:?}", e);
    }
    fs::init_fs(fs);

    let mut sector_buffer = [0u8; 512];
    let mut ata = unsafe { AtaDisk::new() };
    unsafe {
        ata.read_disk_sector(0, &mut sector_buffer).expect("Failed to read sector");
    }

    // Set up the test program
    let user_prog_frame = frame_allocator.allocate_frame().expect("Failed to allocate frame for user program");
    let user_prog_virt_addr = phys_mem_offset + user_prog_frame.start_address().as_u64();
    let user_prog_ptr = user_prog_virt_addr.as_mut_ptr::<u8>();
    let test_program: [u8; 36] = [
        0xB8, 0x01, 0x00, 0x00, 0x00,  // mov eax, 1
        0x48, 0xBF, 0x1F, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00,  // mov rdi, 0x10001F
        0xBE, 0x05, 0x00, 0x00, 0x00,  // mov esi, 5
        0xCD, 0x80,                    // int 0x80
        0xB8, 0x02, 0x00, 0x00, 0x00,  // mov eax, 2
        0x31, 0xFF,                    // xor edi, edi
        0xCD, 0x80,                    // int 0x80
        0x48, 0x65, 0x6C, 0x6C, 0x6F   // "Hello"
    ];
    unsafe {
        for i in 0..test_program.len() {
            *user_prog_ptr.add(i) = test_program[i];
        }
        asm!("mfence");
    }
    let user_prog_page = Page::containing_address(VirtAddr::new(0x100000));
    let user_prog_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    unsafe {
        mapper.map_to(user_prog_page, user_prog_frame, user_prog_flags, &mut frame_allocator)
            .expect("Failed to map user program")
            .flush();
        flush(VirtAddr::new(0x100000));
    }

    // Allocate and map user stack
    let stack_frame = frame_allocator
        .allocate_frame()
        .expect("Failed to allocate frame for user stack");
    let stack_page = Page::containing_address(VirtAddr::new(0x300000));
    let stack_flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::WRITABLE;
    unsafe {
        mapper
            .map_to(stack_page, stack_frame, stack_flags, &mut frame_allocator)
            .expect("Failed to map user stack")
            .flush();
    }

    let stack_frame2 = frame_allocator.allocate_frame().expect("Failed to allocate second frame for user stack");
    let stack_page2 = Page::containing_address(VirtAddr::new(0x301000));
    unsafe {
        mapper.map_to(stack_page2, stack_frame2, stack_flags, &mut frame_allocator)
            .expect("Failed to map second user stack page")
            .flush();
    }

    // Initialize the process queue
    unsafe {
        PROCESS_QUEUE.lock().replace(Vec::new());
        if let Some(queue) = PROCESS_QUEUE.lock().as_mut() {
            queue.push(UserProcess {
                id: ProcessId::new(1),
                entry_point: 0x100000,
                stack_pointer: 0x301ff0,
                state: UserProcessState::Ready,
            });
            // Add more user-space processes here if desired
        }
    }

    println!("[Info] Done.");
    SYSLOG_SINK.log("Initialized Main Kernel Successfully\n", LogLevel::Info);

    WRITER.lock().clear_everything();
    WRITER.lock().set_colors(Color16::White, Color16::Black);

    let _ = crate::keyboard::commands::init_commands();

    // Spawn kernel processes (e.g., keyboard, process_one, process_two)
    let _keyboard_pid = spawn_process(|_state| Box::pin(keyboard::print_keypresses()), false);
    let _process1_pid = spawn_process(|state| Box::pin(process_one(state)), false);
    let _process2_pid = spawn_process(|state| Box::pin(process_two(state)), false);

    serial_println!("[Info] Starting executor...");
    run_executor();
}

/// This function is called on panic.
#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    kernel::hlt_loop();
}

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    kernel::hlt_loop();
}