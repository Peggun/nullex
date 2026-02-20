//!
//! src/task/mod.rs
//! 
//! Module definition for the task handling for the kernel.
//! 

pub mod executor;
pub mod keyboard;

use alloc::{boxed::Box, string::String, sync::Arc};
use core::{
	fmt::Debug,
	future::Future,
	pin::Pin,
	sync::atomic::AtomicBool,
	task::{Context, Poll}
};

use crossbeam_queue::ArrayQueue;
use futures::task::AtomicWaker;
use hashbrown::HashMap;

use crate::utils::oncecell::spin::OnceCell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
/// Wrapper for a process id.
pub struct ProcessId(u64);

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

/// Struct to represent an open file in a process
pub struct OpenFile {
	/// The path to the open file.
	pub path: String,
	/// The current read offset to the open file.
	pub offset: usize
}

#[expect(clippy::type_complexity)]
/// Structure representing all information of a current processes state.
pub struct ProcessState {
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
	/// The File Descriptor to the `OpenFile` mapping.
	pub open_files: HashMap<u32, OpenFile>,
	/// The next available file descriptor.
	pub next_fd: u32,
}

impl Process {
	/// Creates a new process.
	pub fn new(state: Arc<ProcessState>) -> Process {
		let future = (state.future_fn)(state.clone());
		Process {
			state,
			future,
			open_files: HashMap::new(),
			next_fd: 0 // start file descriptors at 0
		}
	}

	/// Tries to get the final result and signs the task up for a callback if its still pending.
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

/// A yield future that yields control back to the executor once before
/// completing.
pub struct YieldNow {
	yielded: bool
}

impl Future for YieldNow {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
		if self.yielded {
			Poll::Ready(())
		} else {
			self.yielded = true;
			cx.waker().wake_by_ref();
			Poll::Pending
		}
	}
}

/// Yields control to the scheduler.
pub async fn yield_now() {
	YieldNow {
		yielded: false
	}
	.await
}
