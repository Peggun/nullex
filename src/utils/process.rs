use alloc::{boxed::Box, sync::Arc};
use core::{future::Future, pin::Pin, sync::atomic::AtomicBool};

use conquer_once::spin::OnceCell;
use futures::task::AtomicWaker;

use crate::task::{Process, ProcessId, ProcessState, executor::EXECUTOR};

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
/// 	  |state| Box::pin(process_one(state)) as Pin<Box<dyn Future<Output = i32>>>,
/// 	  false
/// 	);
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
