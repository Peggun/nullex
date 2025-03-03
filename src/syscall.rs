// syscall.rs

/*
Syscall module for the kernel.
*/

use alloc::sync::Arc;

use crate::{fs, println, serial, serial_println, task::{executor::{CURRENT_PROCESS, EXECUTOR}, Process, ProcessState}};

// System call IDs (for future expansion, though direct function calls are used here for simplicity)
pub const SYS_PRINT: u32 = 1;
pub const SYS_EXIT: u32 = 2;
pub const SYS_FORK: u32 = 3;

// System call handler function (simplified for this example - directly calls kernel functions)
pub fn syscall(syscall_id: u32, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i32 {
    match syscall_id {
        0 => {
            // Handle invalid syscall ID
            -1 // Error code
        }
        4..=u32::MAX => {
            // Handle other syscall IDs
            -1 // Error code
        }
        SYS_PRINT => {
            let ptr = arg1 as *const u8;
            let len = arg2 as usize;
            // Basic, unsafe conversion from raw pointer to &str.
            let s = unsafe { core::str::from_raw_parts(ptr, len) };
            sys_print(s); // Call the actual print function
            0 // Success code
        }
        SYS_EXIT => {
            let exit_code = arg1 as i32;
            sys_exit(exit_code); // Call the actual exit function
            0 // Not reached, as sys_exit terminates the process
        }
        SYS_FORK => {
            sys_fork() // Call the actual fork function
        }
    }
}

// --- Syscall implementations (kernel-level functions) ---

// Process management

pub fn sys_fork() -> i32 {
    serial_println!("sys_fork called");
    // Get the current process's immutable state from CURRENT_PROCESS.
    let current_state = {
        let locked = CURRENT_PROCESS.lock();
        locked.as_ref().expect("No current process during sys_fork").clone()
    };
    
    serial_println!("sys_fork got current process state");

    // Clone the future function directly from the ProcessState.
    let future_fn_clone = current_state.future_fn.clone();

    serial_println!("sys_fork cloned future function");

    let mut executor = EXECUTOR.lock();

    serial_println!("sys_fork got executor lock");

    let child_pid = executor.create_pid();
    
    serial_println!("sys_fork created child PID");

    // Create the child process state with the cloned future function.
    let child_state = Arc::new(ProcessState {
        id: child_pid,
        is_child: true,
        future_fn: future_fn_clone,
    });

    serial_println!("sys_fork created child state");

    let child_process = Process::new(child_state);

    serial_println!("sys_fork created child process");

    executor.spawn_process(child_process);

    serial_println!("sys_fork spawned child process");
    
    child_pid.get() as i32
}

pub fn sys_wait() -> i32 {
    // TODO: Implement waiting for any child process to terminate.
    0
}

// File and filesystem operations

pub fn sys_open() -> i32 {
    // TODO: Implement opening a file.
    0
}

pub fn sys_read() -> i32 {
    // TODO: Implement reading from a file descriptor.
    0
}

pub fn sys_write() -> i32 {
    // TODO: Implement writing to a file descriptor.
    0
}

pub fn sys_close() -> i32 {
    // TODO: Implement closing a file descriptor.
    0
}

// Special syscalls already partially implemented

pub fn sys_print(s: &str) {
    println!("{}", s);
}

pub fn sys_exit(exit_code: i32) -> ! {
    println!("Process exiting with code: {}", exit_code);
    // In a real system, this would perform process cleanup and remove the process from scheduling.
    // For this simplified example, we simulate exit by panicking.
    panic!("sys_exit called - process should terminate (simplified behavior)")
}

pub fn sys_exec() {
        
}