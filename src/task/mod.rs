extern crate alloc;

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

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures::task::AtomicWaker;
use hashbrown::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProcessId(u64);

impl ProcessId {
	pub fn new(id: u64) -> Self {
		ProcessId(id)
	}

	pub fn get(&self) -> u64 {
		self.0
	}
}

// Struct to represent an open file in a process
pub struct OpenFile {
	pub path: String,
	pub offset: usize // current read offset
}

pub struct ProcessState {
	pub id: ProcessId,
	pub is_child: bool,
	pub future_fn:
		Arc<dyn Fn(Arc<ProcessState>) -> Pin<Box<dyn Future<Output = i32>>> + Send + Sync>,
	pub queued: AtomicBool,
	pub scancode_queue: OnceCell<ArrayQueue<u8>>,
	pub waker: AtomicWaker
}

pub struct Process {
	pub state: Arc<ProcessState>,
	pub future: Pin<Box<dyn Future<Output = i32>>>,
	pub open_files: HashMap<u32, OpenFile>, // file descriptor to OpenFile mapping
	pub next_fd: u32                        // next available file descriptor
}

impl Process {
	pub fn new(state: Arc<ProcessState>) -> Process {
		let future = (state.future_fn)(state.clone());
		Process {
			state,
			future,
			open_files: HashMap::new(),
			next_fd: 0 // start file descriptors at 0
		}
	}

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
	pub yielded: bool
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
