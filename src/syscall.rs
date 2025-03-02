// syscall.rs

/*
Syscall module for the kernel.
*/

use crate::{fs, println, serial_println};
// Assuming EXECUTOR is globally accessible
use crate::fs::ramfs::Permission;

// System call IDs (for future expansion, though direct function calls are used here for simplicity)
pub const SYS_PRINT: u32 = 1;
pub const SYS_EXIT: u32 = 2;
pub const SYS_GETPID: u32 = 3;

// System call handler function (simplified for this example - directly calls kernel functions)
pub fn syscall(syscall_id: u32, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> i32 {
    match syscall_id {
        SYS_PRINT => {
            let ptr = arg1 as *const u8;
            let len = arg2 as usize;
            // Basic, unsafe conversion from raw pointer to &str.
            let s = unsafe { core::str::from_raw_parts(ptr, len) };
            println!("{}", s);
            0 // Success code
        }
        SYS_EXIT => {
            let exit_code = arg1 as i32;
            sys_exit(exit_code); // Call the actual exit function
            0 // Not reached, as sys_exit terminates the process
        }
        SYS_GETPID => sys_getpid() as i32, // Return PID as i32
        _ => {
            serial_println!("Unknown syscall ID: {}", syscall_id);
            -1 // Error code for unknown syscall
        }
    }
}

// --- Syscall implementations (kernel-level functions) ---

// Process management

pub fn sys_fork() -> i32 {
    // TODO: Implement process forking.
    0
}

pub fn sys_execve() -> i32 {
    // TODO: Implement executing a new program.
    0
}

pub fn sys_wait() -> i32 {
    // TODO: Implement waiting for any child process to terminate.
    0
}

pub fn sys_waitpid() -> i32 {
    // TODO: Implement waiting for a specific child process.
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

pub fn sys_lseek() -> i32 {
    // TODO: Implement repositioning the file offset.
    0
}

pub fn sys_stat() -> i32 {
    // TODO: Implement retrieving file metadata.
    0
}

pub fn sys_fstat() -> i32 {
    // TODO: Implement retrieving file metadata using a file descriptor.
    0
}

pub fn sys_mkdir(path: &str, mode: u32) -> i32 {
    // Use the ramfs to create a directory with the given path and mode.
    // In this example, we ignore `mode` and grant all permissions.
    fs::with_fs(|fs| {
        fs.create_dir(path, Permission::all()).unwrap();
    });
    0
}

pub fn sys_rmdir(path: &str) -> i32 {
    // TODO: Implement removing a directory.
    0
}

pub fn sys_unlink(path: &str) -> i32 {
    // TODO: Implement deleting a file.
    0
}

// Memory management

pub fn sys_mmap() -> i32 {
    // TODO: Implement mapping files or anonymous memory into the process's address space.
    0
}

pub fn sys_brk() -> i32 {
    // TODO: Implement adjusting the end of the data segment.
    0
}

pub fn sys_sbrk() -> i32 {
    // TODO: Implement incrementing the data segment (heap).
    0
}

// Inter-Process Communication (IPC)

pub fn sys_pipe() -> i32 {
    // TODO: Implement creating a unidirectional data channel (pipe).
    0
}

pub fn sys_dup() -> i32 {
    // TODO: Implement duplicating a file descriptor.
    0
}

pub fn sys_dup2() -> i32 {
    // TODO: Implement duplicating a file descriptor to a specified descriptor number.
    0
}

// User and group management (stubbed for now)

pub fn sys_getuid() -> i32 {
    // TODO: Implement retrieving the user ID.
    0
}

pub fn sys_geteuid() -> i32 {
    // TODO: Implement retrieving the effective user ID.
    0
}

pub fn sys_setuid() -> i32 {
    // TODO: Implement setting the user ID.
    0
}

// Process environment and working directory

pub fn sys_chdir() -> i32 {
    // TODO: Implement changing the current working directory.
    0
}

// Filesystem mounting and device control

pub fn sys_mount() -> i32 {
    // TODO: Implement mounting a filesystem.
    0
}

pub fn sys_ioctl() -> i32 {
    // TODO: Implement device-specific control operations.
    0
}

// Basic process information

pub fn sys_getpid() -> u64 {
    // In a real kernel, this would return the PID of the current process.
    // For this simplified example, we return a fixed placeholder.
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
