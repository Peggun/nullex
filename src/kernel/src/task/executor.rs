// executor.rs

extern crate alloc;

use alloc::{collections::BTreeMap, sync::Arc, task::Wake, vec::Vec};
use core::{
	arch::asm,
	sync::atomic::{AtomicBool, Ordering},
	task::{Poll, Waker}
};

use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;
use spin::{Mutex, mutex::SpinMutex};
use x86_64::{PhysAddr, VirtAddr, instructions::interrupts, structures::paging::Translate};

use super::{Process, ProcessId, ProcessState};
use crate::{println, serial_println};

lazy_static! {
	pub static ref CURRENT_PROCESS: SpinMutex<Option<Arc<ProcessState>>> = SpinMutex::new(None);
}

lazy_static! {
	pub static ref EXECUTOR: spin::Mutex<Executor> = spin::Mutex::new(Executor::new());
}

lazy_static! {
	pub static ref CURRENT_PID: Mutex<Option<ProcessId>> = Mutex::new(None);
	pub static ref PROCESS_QUEUE: Mutex<Option<Vec<UserProcess>>> = Mutex::new(None);
}

pub static mut EXECUTOR_STACK: [u8; 4096] = [0; 4096];
pub static SCHEDULER_TICK_PENDING: AtomicBool = AtomicBool::new(false);

// User process structure
pub struct UserProcess {
	pub id: ProcessId,
	pub entry_point: usize,
	pub stack_pointer: usize,
	pub kernel_stack_top: usize,
	pub state: UserProcessState
}

#[derive(PartialEq)]
pub enum UserProcessState {
	Ready,
	Running,
	Terminated
}

#[derive(PartialEq)]
pub enum KernelProcessState {
	Ready,
	Running,
	Terminated
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
		let process = self.processes.get(&pid).unwrap().lock();
		process.state.queued.store(true, Ordering::Release);
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

	pub fn end_process(&mut self, pid: ProcessId, _exit_code: i32) {
		let process_arc = self.processes.get(&pid).unwrap();
		let process = process_arc.lock();
		let pid_to_remove = pid;
		drop(process);
		self.processes.remove(&pid_to_remove);
		self.waker_cache.remove(&pid_to_remove);
	}
}

pub struct ProcessWaker {
	pub pid: ProcessId,
	pub process_queue: Arc<ArrayQueue<ProcessId>>,
	pub state: Arc<ProcessState>
}

impl ProcessWaker {
	pub fn wake_process(&self) {
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

pub fn allocate_kernel_stack() -> usize {
	const STACK_SIZE: usize = 5 * 4096;
	let stack_bottom = unsafe {
		alloc::alloc::alloc_zeroed(alloc::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap())
	} as usize;
	let stack_top = stack_bottom + STACK_SIZE;
	assert!(stack_top % 16 == 0);
	stack_top
}

pub fn deallocate_kernel_stack(stack_top: usize) {
	const STACK_SIZE: usize = 5 * 4096;
	let stack_bottom = stack_top - STACK_SIZE;
	unsafe {
		alloc::alloc::dealloc(
			stack_bottom as *mut u8,
			alloc::alloc::Layout::from_size_align(STACK_SIZE, 16).unwrap()
		)
	};
}

/// Switch to a user-space process using GDT selectors
pub unsafe fn switch_to_process(process: &UserProcess) {
	const USER_CODE_SEL: u16 = 0x1B;
	const USER_DATA_SEL: u16 = 0x23;

	// disable interrupts
	interrupts::disable();

	// Set TSS to use the process's kernel stack
	unsafe {
		crate::gdt::TSS.privilege_stack_table[0] = VirtAddr::new(process.kernel_stack_top as u64);
		serial_println!(
			"[Debug] TSS.RSP0 set to {:#x}",
			crate::gdt::TSS.privilege_stack_table[0].as_u64()
		);
	}

	unsafe {
		asm!(
			// Load data segments
			"mov ax, {data_sel}",
			"mov ds, ax",
			"mov es, ax",
			"mov fs, ax",
			"mov gs, ax",

			// Push SS (as 64-bit value)
			"mov rax, {data_sel}",
			"push rax",

			// Push user RSP
			"push {user_stack}",

			// Push RFLAGS (with interrupts enabled)
			"pushfq",
			"pop rax",
			"or rax, 0x200",
			"push rax",

			// Push CS (as 64-bit value)
			"mov rax, {code_sel}",
			"push rax",

			// Push RIP (user entry point)
			"push {user_entry}",

			// Return to user mode
			"iretq",
			data_sel = const USER_DATA_SEL,
			user_stack = in(reg) process.stack_pointer,
			code_sel = const USER_CODE_SEL,
			user_entry = in(reg) process.entry_point,
			options(noreturn)
		);
	}
}

/// Retrieve the kernel stack pointer
pub fn kernel_stack_top() -> usize {
	unsafe { crate::gdt::KERNEL_STACK_TOP }
}

pub fn translate_user_virtual_address(addr: VirtAddr) -> Option<PhysAddr> {
	let page_table_ptr = x86_64::registers::control::Cr3::read()
		.0
		.start_address()
		.as_u64() as *mut _;
	let page_table = unsafe { &mut *page_table_ptr };
	let mapper =
		unsafe { x86_64::structures::paging::OffsetPageTable::new(page_table, VirtAddr::new(0)) };
	mapper.translate_addr(addr)
}

/// Executor loop to run processes
pub fn run_executor() -> ! {
	loop {
		unsafe {
			if let Some(queue) = PROCESS_QUEUE.lock().as_mut() {
				queue.retain(|p| {
					if p.state == UserProcessState::Terminated {
						deallocate_kernel_stack(p.kernel_stack_top);
						false
					} else {
						true
					}
				});
				if let Some(process) = queue
					.iter_mut()
					.find(|p| p.state == UserProcessState::Ready)
				{
					process.state = UserProcessState::Running;
					*CURRENT_PID.lock() = Some(process.id);
					serial_println!("[Info] Switching to process {}", process.id.get());
					switch_to_process(process);
				} else {
					serial_println!("[Info] No ready processes, idling...");
					asm!("hlt");
				}
			} else {
				panic!("Process queue not initialized");
			}
		}
	}
}

/// Run both user and kernel executors
pub fn run_combined_executor() -> ! {
	loop {
		// -- First, run kernel processes from the EXECUTOR --
		{
			let mut executor = EXECUTOR.lock();
			if let Some(pid) = executor.process_queue.pop() {
				if let Some(process_arc) = executor.processes.get(&pid) {
					let mut process = process_arc.lock();
					process.state.queued.store(false, Ordering::Release);
					let waker = ProcessWaker::new(
						pid,
						executor.process_queue.clone(),
						process.state.clone()
					);
					let mut context = core::task::Context::from_waker(&waker);
					if let Poll::Ready(result) = process.poll(&mut context) {
						serial_println!(
							"[Info] Kernel Process {} completed with {:?}",
							pid.get(),
							result
						);
						let exit_code = result.unwrap_or(-1);
						drop(process);
						executor.end_process(pid, exit_code);
						// Continue immediately to schedule next kernel process.
						continue;
					} else {
						// If not ready, reinsert the process into the kernel queue.
						if executor.process_queue.push(pid).is_err() {
							serial_println!("[Warning] Executor process queue full");
						}
					}
				}
			}
		}

		// -- Then, run user processes from PROCESS_QUEUE --
		unsafe {
			if let Some(queue) = PROCESS_QUEUE.lock().as_mut() {
				if let Some(user_process) = queue
					.iter_mut()
					.find(|p| p.state == UserProcessState::Ready)
				{
					user_process.state = UserProcessState::Running;
					*CURRENT_PID.lock() = Some(user_process.id);
					serial_println!("[Info] Switching to user process {}", user_process.id.get());
					switch_to_process(user_process);
					// Note: switch_to_process never returns.
				} else {
					serial_println!("[Info] No ready user processes, idling...");
					asm!("hlt");
				}
			} else {
				panic!("Process queue not initialized");
			}
		}
	}
}
