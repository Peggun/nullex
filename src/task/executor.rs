//!
//! executor.rs
//! 
//! Process execution logic for the kernel.
//! 

use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use core::{sync::atomic::Ordering, task::Waker};

use crossbeam_queue::ArrayQueue;

use super::{Process, ProcessId, ProcessState};
use crate::{lazy_static, println, serial_println, utils::mutex::SpinMutex};

lazy_static! {
	/// Static reference to the current process that is running.
	pub static ref CURRENT_PROCESS: SpinMutex<Option<Arc<ProcessState>>> = SpinMutex::new(None);
	/// Static reference to the current executor that the kernel is running.
	pub static ref EXECUTOR: SpinMutex<Executor> = SpinMutex::new(Executor::new());
}

/// Pointer to the current process.
pub static mut CURRENT_PROCESS_GUARD: *mut Process = core::ptr::null_mut();

/// The process executor of the kernel.
pub struct Executor {
	/// Tree map showing all mapped processes.
	pub processes: BTreeMap<ProcessId, Arc<SpinMutex<Process>>>,
	/// The queue of all processes waiting to run.
	pub process_queue: Arc<ArrayQueue<ProcessId>>,
	/// Cache of all wakers for a process.
	pub waker_cache: BTreeMap<ProcessId, Waker>,
	/// Next `ProcessId` to be run.
	pub next_pid: ProcessId
}

impl Executor {
	/// Creates a new process executor
	pub fn new() -> Self {
		Executor {
			processes: BTreeMap::new(),
			process_queue: Arc::new(ArrayQueue::new(100)),
			waker_cache: BTreeMap::new(),
			next_pid: ProcessId::new(0)
		}
	}


	/// Spawns a new process.
	pub fn spawn_process(&mut self, process: Process) {
		let pid = process.state.id;
		let process_arc = Arc::new(SpinMutex::new(process));
		if self.processes.insert(pid, process_arc).is_some() {
			panic!("process with same ID already in processes");
		}
		self.process_queue.push(pid).expect("queue full");
	}

	/// Sleeps the executor if there are no pending processes.
	pub fn sleep_if_idle(&self) {
		use x86_64::instructions::interrupts;
		interrupts::disable();
		if self.process_queue.is_empty() {
			interrupts::enable_and_hlt();
		} else {
			interrupts::enable();
		}
	}

	/// Creates a new `Process ID` for a `Process`
	pub fn create_pid(&mut self) -> ProcessId {
		let pid = self.next_pid;
		self.next_pid = ProcessId::new(pid.0 + 1);
		pid
	}

	/// Lists the running processes.
	pub fn list_processes(&self) {
		println!("Running processes:");
		for pid in self.processes.keys() {
			println!("  Process {}", pid.0);
		}
	}

	/// Ends a running process.
	pub fn end_process(&mut self, pid: ProcessId, exit_code: i32) {
		let process_arc = self.processes.get(&pid).unwrap();
		let process = process_arc.lock();
		let pid_to_remove = pid;
		drop(process);
		self.processes.remove(&pid_to_remove);
		self.waker_cache.remove(&pid_to_remove);

		serial_println!("Process {} exited with code: {}", pid.get(), exit_code);
	}
}

impl Default for Executor {
	fn default() -> Self {
		Self::new()
	}
}

/// Structure representing a waker, to be able to 'wake' a process up.
pub struct ProcessWaker {
	/// The `ProcessId` to wake up.
	pub pid: ProcessId,
	/// The list of `ProcessId`'s to wake up.
	pub process_queue: Arc<ArrayQueue<ProcessId>>,
	/// The current state of the process which will be waking up.
	pub state: Arc<ProcessState>
}

impl ProcessWaker {
	/// Wakes the current process inside of `self.pid`
	pub fn wake_process(&self) {
		// use self.state directly no need to lock the process
		if !self.state.queued.swap(true, Ordering::AcqRel)
			&& self.process_queue.push(self.pid).is_err()
		{
			serial_println!(
				"Warning: process_queue full, skipping wake for process {}",
				self.pid.0
			);
			self.state.queued.store(false, Ordering::Release);
		}
	}

	/// Creates a new waker for a `ProcessId`
	pub fn new_waker(
		pid: ProcessId,
		process_queue: Arc<ArrayQueue<ProcessId>>,
		state: Arc<ProcessState>
	) -> Waker {
		Waker::from(Arc::new(ProcessWaker {
			pid,
			process_queue,
			state
		}))
	}
}

impl Wake for ProcessWaker {
	fn wake(self: Arc<Self>) {
		self.wake_process();
	}

	fn wake_by_ref(self: &Arc<Self>) {
		self.wake_process();
	}
}