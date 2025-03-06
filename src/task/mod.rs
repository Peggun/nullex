extern crate alloc;

pub mod executor;
pub mod keyboard;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::AtomicBool;
use core::task::Context;
use hashbrown::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessId(u64);

impl ProcessId {
    pub fn new(id: u64) -> Self {
        ProcessId(id)
    }

    pub fn get(&self) -> u64 {
        self.0
    }
}

// Struct to represent an open file in a process
pub struct OpenFile {
    pub path: String,
    pub offset: usize, // Current read offset
}

pub struct ProcessState {
    pub id: ProcessId,
    pub is_child: bool,
    pub future_fn: Arc<dyn Fn(Arc<ProcessState>) -> Pin<Box<dyn Future<Output = i32>>> + Send + Sync>,
    pub queued: AtomicBool,
}

pub struct Process {
    pub state: Arc<ProcessState>,
    pub future: Pin<Box<dyn Future<Output = i32>>>,
    pub open_files: HashMap<u32, OpenFile>, // File descriptor to OpenFile mapping
    pub next_fd: u32,                       // Next available file descriptor
}

impl Process {
    pub fn new(state: Arc<ProcessState>) -> Process {
        let future = (state.future_fn)(state.clone());
        Process {
            state,
            future,
            open_files: HashMap::new(),
            next_fd: 0, // Start file descriptors at 0
        }
    }

    pub fn poll(&mut self, context: &mut Context) -> core::task::Poll<i32> {
        self.future.as_mut().poll(context)
    }
}

unsafe impl Send for Process {}

/// A future that never completes.
pub struct ForeverPending;

impl Future for ForeverPending {
    type Output = i32;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> core::task::Poll<Self::Output> {
        core::task::Poll::Pending
    }
}