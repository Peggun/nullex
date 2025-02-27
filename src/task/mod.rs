// task/mod.rs
extern crate alloc;

pub mod executor;
pub mod keyboard;

use core::{future::Future, pin::Pin};
use alloc::boxed::Box;

use core::task::{Context, Poll};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessId(u64); // Renamed TaskId to ProcessId

impl ProcessId { // Renamed TaskId to ProcessId
    pub fn new(id: u64) -> Self { // Now takes an initial ID, useful for Executor's PID counter
        ProcessId(id)
    }
}

pub struct Process { // Renamed Task to Process
    pub id: ProcessId, // Renamed id to pid and TaskId to ProcessId, made public for syscalls to access
    future: Pin<Box<dyn Future<Output = i32>>>, // Future now returns an exit code (i32)
}

impl Process { // Renamed Task to Process
    pub fn new(id: ProcessId, future: impl Future<Output = i32> + 'static) -> Process { // Renamed Task to Process and takes ProcessId
        Process {
            id, // Use provided ProcessId
            future: Box::pin(future),
        }
    }

    fn poll(&mut self, context: &mut Context) -> Poll<i32> { // Poll now returns Poll<i32>
        self.future.as_mut().poll(context)
    }
}

unsafe impl Send for Process {}