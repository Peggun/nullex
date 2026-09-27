//!
//! executor.rs
//!
//! Process execution logic for the kernel.

use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use core::{
	sync::atomic::Ordering,
	task::{Context, Poll, Waker}
};

use crossbeam_queue::ArrayQueue;

use crate::{
	error::NullexError,
	lazy_static,
	serial_println,
	sync::mutex::SpinMutex,
	task::{
		executor,
		process::{Process, ProcessId, ProcessState}
	}
};

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
	pub fn spawn_process(&mut self, process: Process) -> Result<(), NullexError> {
		let pid = process.state.id;
		let process_arc = Arc::new(SpinMutex::new(process));
		if self.processes.insert(pid, process_arc).is_some() {
			return Err(NullexError::ProcessAlreadyExists);
		}
		self.process_queue
			.push(pid)
			.map_err(|_| NullexError::ProcessQueueFull)?;
		Ok(())
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
		serial_println!("Running processes:");
		for pid in self.processes.keys() {
			serial_println!("  Process {}", pid.0);
		}
	}

	/// Ends a running process.
	pub fn end_process(&mut self, pid: ProcessId, exit_code: i32) {
		let Some(process_arc) = self.processes.remove(&pid) else {
			serial_println!("end_process: process {} already removed", pid.get());
			return;
		};

		serial_println!(
			"end pid={} strong={} weak={}",
			pid.get(),
			Arc::strong_count(&process_arc.lock().state),
			Arc::weak_count(&process_arc.lock().state),
		);

		self.waker_cache.remove(&pid);

		drop(process_arc);

		serial_println!("Process {} exited with code: {}", pid.get(), exit_code);
	}

	pub fn get_process(&self, pid: ProcessId) -> Option<Arc<SpinMutex<Process>>> {
		self.processes.get(&pid).cloned()
	}
}

impl Default for Executor {
	fn default() -> Self {
		Self::new()
	}
}

pub fn run_executor(exit_on_pid: Option<ProcessId>) -> ! {
	let process_queue = EXECUTOR.lock().process_queue.clone();

	loop {
		if let Some(pid) = process_queue.pop() {
			serial_println!("executing pid={}", pid.get());

			if let Some(process_arc) = EXECUTOR.lock().get_process(pid) {
				process_arc
					.lock()
					.state
					.queued
					.store(false, Ordering::Release);
			}

			let process_arc = {
				let executor = EXECUTOR.lock();
				executor.get_process(pid).clone()
			};

			if let Some(process_arc) = process_arc {
				*CURRENT_PROCESS.lock() = Some(process_arc.lock().state.clone());

				let mut process = process_arc.lock();
				let process_state = process.state.clone();

				unsafe {
					CURRENT_PROCESS_GUARD = &mut *process as *mut Process;
				}

				let waker = {
					let mut executor = EXECUTOR.lock();
					executor
						.waker_cache
						.entry(pid)
						.or_insert_with(|| {
							ProcessWaker::new_waker(pid, process_queue.clone(), process_state)
						})
						.clone()
				};

				let mut context = Context::from_waker(&waker);
				let result = process.future.as_mut().poll(&mut context);

				x86_64::instructions::interrupts::enable();

				unsafe {
					CURRENT_PROCESS_GUARD = core::ptr::null_mut();
				}

				if let Poll::Ready(exit_code) = result {
					let mut executor = EXECUTOR.lock();
					executor.processes.remove(&pid);
					executor.waker_cache.remove(&pid);
					serial_println!("Process {} exited with code: {}", pid.get(), exit_code);
					drop(executor);

					#[cfg(feature = "test")]
					if let Some(target_pid) = exit_on_pid {
						if pid == target_pid {
							serial_println!("[KTEST] Test process exited. Exiting QEMU.");
							crate::qemu_exit(exit_code as u32);
						}
					}
				}
				*CURRENT_PROCESS.lock() = None;
			}
		} else {
			EXECUTOR.lock().sleep_if_idle();
		}
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
		serial_println!(
			"wake pid={} state={:p}",
			self.pid.get(),
			Arc::as_ptr(&self.state),
		);

		serial_println!("queued field = {:p}", &self.state.queued as *const _);

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
