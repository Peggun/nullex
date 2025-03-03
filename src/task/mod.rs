extern crate alloc;

pub mod executor;
pub mod keyboard;

use alloc::boxed::Box;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::task::Context;

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

// Move the future function into ProcessState.
pub struct ProcessState {
    pub id: ProcessId,
    pub is_child: bool,
    pub future_fn: Arc<dyn Fn(Arc<ProcessState>) -> Pin<Box<dyn Future<Output = i32>>> + Send + Sync>,
}

// Process structure now only stores the state and the future.
pub struct Process {
    pub state: Arc<ProcessState>,
    pub future: Pin<Box<dyn Future<Output = i32>>>,
}

impl Process {
    // Update the constructor to use the future_fn stored in state.
    pub fn new(state: Arc<ProcessState>) -> Process {
        let future = (state.future_fn)(state.clone());
        Process {
            state,
            future,
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