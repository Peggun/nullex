use alloc::boxed::Box;
use alloc::sync::Arc;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::AtomicBool;
use crate::task::{Process, ProcessState};
use crate::task::executor::EXECUTOR;
use crate::task::ProcessId;

/// Spawns a process using the provided future function.
/// 
/// # Arguments
/// 
/// * `future_fn` - A closure that receives a process state and returns a boxed future.
/// * `is_child` - A flag indicating if the process is a child.
/// 
/// # Returns
/// 
/// The ProcessId of the newly spawned process.
pub fn spawn_process<F>(future_fn: F, is_child: bool) -> ProcessId
where
    F: Fn(Arc<ProcessState>) -> Pin<Box<dyn Future<Output = i32>>> + Send + Sync + 'static,
{
    // Lock the executor and create a new PID.
    let mut executor = EXECUTOR.lock();
    let pid = executor.create_pid();

    // Create the process state.
    let state = Arc::new(ProcessState {
        id: pid,
        is_child,
        future_fn: Arc::new(future_fn),
        queued: AtomicBool::new(false),
    });

    // Construct the process.
    let process = Process::new(state);
    // Spawn the process.
    executor.spawn_process(process);
    pid
}
