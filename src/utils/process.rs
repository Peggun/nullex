//!
//! process.rs
//! 
//! Utilities for process handling for the kernel.
//! 

use alloc::{boxed::Box, sync::Arc};
use core::{future::Future, pin::Pin, sync::atomic::{AtomicBool, Ordering}};

use futures::task::AtomicWaker;

use crate::{
	apic::{APIC_TICK_COUNT, APIC_TPS}, task::{Process, ProcessId, ProcessState, executor::EXECUTOR, yield_now}, utils::oncecell::cell::OnceCell
};

/// Spawns a process using the provided future function.
///
/// # Arguments
///
/// * `future_fn` - A closure that receives a process state and returns a boxed
///   future.
/// * `is_child` - A flag indicating if the process is a child.
///
/// # Returns
///
/// # Example
/// ```rs
/// 
/// // Create a process using spawn_process.
/// let _process1_pid = spawn_process(
///     |state| Box::pin(process_one(state)) as Pin<Box<dyn Future<Output = i32>>>,
///     false
/// );
/// ```
///
/// The ProcessId of the newly spawned process.
pub fn spawn_process<F>(future_fn: F, is_child: bool) -> ProcessId
where
	F: Fn(Arc<ProcessState>) -> Pin<Box<dyn Future<Output = i32>>> + Send + Sync + 'static
{
	// lock the executor and create a new PID.
	let mut executor = EXECUTOR.lock();
	let pid = executor.create_pid();

	// create the process state.
	let state = Arc::new(ProcessState {
		id: pid,
		is_child,
		future_fn: Arc::new(future_fn),
		queued: AtomicBool::new(false),
		scancode_queue: OnceCell::uninit(),
		waker: AtomicWaker::new()
	});

	// construct the process.
	let process = Process::new(state);
	// spawn the process.
	executor.spawn_process(process);
	pid
}

#[allow(unused)]
/// # Safety
/// Should NEVER be used in kernel space. only like a API for syscalls and user space later.
async unsafe fn sleep(ms: u64) {
    let tps = APIC_TPS.load(Ordering::Relaxed);
    let now = APIC_TICK_COUNT.load(Ordering::Relaxed);
    let ticks = (ms * tps) / 1000;
    let then = now + ticks;

    while APIC_TICK_COUNT.load(Ordering::Relaxed) < then {
        yield_now().await;
    }
}