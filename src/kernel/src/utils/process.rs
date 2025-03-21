use alloc::{boxed::Box, sync::Arc};
use core::{future::Future, pin::Pin, sync::atomic::AtomicBool};

use crate::{
	errors::KernelError,
	task::{Process, ProcessId, ProcessState, executor::EXECUTOR}
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
/// The ProcessId of the newly spawned process.
///
/// # Example
/// ```rust
/// // Create a process using spawn_process.
/// let _process1_pid = spawn_process(
/// 	|state| {
/// 		Box::pin(process_one(state)) as Pin<Box<dyn Future<Output = Result<i32, KernelError>>>>
/// 	},
/// 	false
/// );
/// ```
pub fn spawn_process<F>(future_fn: F, is_child: bool) -> ProcessId
where
	F: Fn(Arc<ProcessState>) -> Pin<Box<dyn Future<Output = Result<i32, KernelError>>>>
		+ Send
		+ Sync
		+ 'static
{
	// Lock the executor and create a new PID.
	let mut executor = EXECUTOR.lock();
	let pid = executor.create_pid();

	// Create the process state.
	let state = Arc::new(ProcessState {
		id: pid,
		is_child,
		future_fn: Arc::new(future_fn),
		queued: AtomicBool::new(false)
	});

	// Construct the process.
	let process = Process::new(state);
	// Spawn the process.
	executor.spawn_process(process);
	pid
}
