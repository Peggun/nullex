// executor.rs

extern crate alloc;

use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use core::{sync::atomic::Ordering, task::Waker};

use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;
use spin::mutex::SpinMutex;

use super::{Process, ProcessId, ProcessState};
use crate::{println, serial_println};

lazy_static! {
	pub static ref CURRENT_PROCESS: SpinMutex<Option<Arc<ProcessState>>> = SpinMutex::new(None);
}

pub static mut CURRENT_PROCESS_GUARD: *mut Process = core::ptr::null_mut();

pub struct Executor {
	pub processes: BTreeMap<ProcessId, Arc<SpinMutex<Process>>>,
	pub process_queue: Arc<ArrayQueue<ProcessId>>,
	pub waker_cache: BTreeMap<ProcessId, Waker>,
	pub next_pid: ProcessId
}

impl Executor {
	pub fn new() -> Self {
		Executor {
			processes: BTreeMap::new(),
			process_queue: Arc::new(ArrayQueue::new(100)),
			waker_cache: BTreeMap::new(),
			next_pid: ProcessId::new(0)
		}
	}

	pub fn spawn_process(&mut self, process: Process) {
		let pid = process.state.id;
		let process_arc = Arc::new(SpinMutex::new(process));
		if self.processes.insert(pid, process_arc).is_some() {
			panic!("process with same ID already in processes");
		}
		self.process_queue.push(pid).expect("queue full");
	}

	pub fn sleep_if_idle(&self) {
		use x86_64::instructions::interrupts;
		interrupts::disable();
		if self.process_queue.is_empty() {
			interrupts::enable_and_hlt();
		} else {
			interrupts::enable();
		}
	}

	pub fn create_pid(&mut self) -> ProcessId {
		let pid = self.next_pid;
		self.next_pid = ProcessId::new(pid.0 + 1);
		pid
	}

	pub fn list_processes(&self) {
		println!("Running processes:");
		for (pid, _) in &self.processes {
			println!("  Process {}", pid.0);
		}
	}
}

pub struct ProcessWaker {
	pub pid: ProcessId,
	pub process_queue: Arc<ArrayQueue<ProcessId>>,
	pub state: Arc<ProcessState> // Added to store process state
}

impl ProcessWaker {
	pub fn wake_process(&self) {
		// Use self.state directly, no need to lock the process
		if !self.state.queued.swap(true, Ordering::AcqRel) {
			if self.process_queue.push(self.pid).is_err() {
				serial_println!(
					"Warning: process_queue full, skipping wake for process {}",
					self.pid.0
				);
				self.state.queued.store(false, Ordering::Release);
			}
		}
	}

	pub fn new(
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

lazy_static! {
	pub static ref EXECUTOR: spin::Mutex<Executor> = spin::Mutex::new(Executor::new());
}
