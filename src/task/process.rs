use alloc::{boxed::Box, string::String, sync::Arc, vec::Vec};
use core::{pin::Pin, sync::atomic::AtomicBool, task::Context};

use crossbeam_queue::ArrayQueue;
use futures::task::AtomicWaker;
use hashbrown::HashMap;

use crate::{
	arch::x86_64::user::setup_user_stack,
	error::NullexError,
	gdt::{user_code_selector, user_data_selector},
	process::{load_segment, parse_elf},
	serial_println,
	sync::oncecell::spin::OnceCell,
	task::{
		KERNEL_STACK_SIZE,
		UserProcessFuture,
		address_space::{AddressSpace, Cr3Guard},
		context::UserContext,
		files::OpenFile
	}
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Wrapper for a process id.
pub struct ProcessId(pub u64);

impl ProcessId {
	/// Creates a new `ProcessId` with the specified id.
	pub fn new(id: u64) -> Self {
		ProcessId(id)
	}

	/// Returns the `ProcessId`'s id.
	pub fn get(&self) -> u64 {
		self.0
	}
}

#[expect(clippy::type_complexity)]
/// Structure representing all information of a current processes state.
pub struct ProcessState {
	//pub name: String // to add
	/// The current `ProcessId`
	pub id: ProcessId,
	/// Whether or not the running process is a child of another process.
	pub is_child: bool,
	/// The function that this process will be running.
	pub future_fn:
		Arc<dyn Fn(Arc<ProcessState>) -> Pin<Box<dyn Future<Output = i32>>> + Send + Sync>,
	/// Whether or not is it in the queued inside of the executor.
	pub queued: AtomicBool,
	/// Scancode queue incase some functions need the keyboard.
	pub scancode_queue: OnceCell<ArrayQueue<u8>>,
	/// Waker for functions that need the process now.
	pub waker: AtomicWaker
}

/// Structure representing a process running in the kernel.
pub struct Process {
	/// Current state of the process running.
	pub state: Arc<ProcessState>,
	/// The code that is running inside of the process.
	pub future: Pin<Box<dyn Future<Output = i32>>>,
	/// Registers saved from User Processes
	pub context: UserContext,
	/// Address space for the process
	pub address_space: Option<AddressSpace>,
	/// The File Descriptor to the `OpenFile` mapping.
	pub open_files: HashMap<u32, OpenFile>,
	/// The next available file descriptor.
	pub next_fd: u32,

	pub kernel_stack: Vec<u8>,
	pub kernel_stack_top: u64
}

impl Process {
	/// Creates a new process.
	pub fn new(state: Arc<ProcessState>) -> Result<Process, NullexError> {
		let future = (state.future_fn)(state.clone());
		let mut kernel_stack = vec![0u8; KERNEL_STACK_SIZE];
		kernel_stack.fill(0xCC);
		let kernel_stack_top = kernel_stack.as_ptr() as u64 + KERNEL_STACK_SIZE as u64;

		Ok(Process {
			state,
			future,
			context: UserContext::default(),
			address_space: None,
			open_files: HashMap::new(),
			next_fd: 3, // start file descriptors at 3 because 0 - stdin, 1 - stdout and 2 - stderr
			kernel_stack,
			kernel_stack_top
		})
	}

	/// Creates a new process from an ELF binary.
	pub fn from_elf(
		state: Arc<ProcessState>,
		elf_bytes: &[u8],
		args: Vec<String>,
		envs: Vec<String>
	) -> Result<Process, NullexError> {
		serial_println!("from_elf: begin pid={}", state.id.get());
		let _cr3_guard = unsafe { Cr3Guard::enter_kernel() };
		let elf = parse_elf(elf_bytes)?;

		let mut address_space = AddressSpace::new()?;

		for seg in &elf.segments {
			load_segment(&mut address_space, elf_bytes, seg)?;
		}

		let stack_top = unsafe { setup_user_stack(&mut address_space, args, envs) }?;

		let mut context = UserContext::default();
		context.rip = elf.entry;
		context.rsp = stack_top;
		context.cs = user_code_selector() as u64;
		context.ss = user_data_selector() as u64;
		context.rflags = 0x202;

		let future = Box::pin(UserProcessFuture);
		let mut kernel_stack = vec![0u8; KERNEL_STACK_SIZE];
		kernel_stack.fill(0xCC);
		let kernel_stack_top = kernel_stack.as_ptr() as u64 + KERNEL_STACK_SIZE as u64;

		serial_println!("from_elf: end pid={}", state.id.get());

		Ok(Process {
			state,
			future,
			context,
			address_space: Some(address_space),
			open_files: HashMap::new(),
			next_fd: 3,
			kernel_stack,
			kernel_stack_top
		})
	}

	/// Tries to get the final result and signs the task up for a callback if
	/// its still pending.
	pub fn poll(&mut self, context: &mut Context) -> core::task::Poll<i32> {
		self.future.as_mut().poll(context)
	}
}
unsafe impl Send for Process {}
