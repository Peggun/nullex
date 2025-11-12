// code from https://github.com/rust-embedded-community/pc-keyboard
// license in THIRD_PARTY_LICENSE


use core::task::Poll;

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use futures::{Stream, task::AtomicWaker};

use crate::println;

pub static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
pub static WAKER: AtomicWaker = AtomicWaker::new();

pub(crate) fn add_scancode(scancode: u8) {
	if let Ok(queue) = SCANCODE_QUEUE.try_get() {
		if queue.push(scancode).is_err() {
			println!(
				"WARNING: scancode queue full; dropping keyboard input {}",
				scancode
			);
		} else {
			WAKER.wake();
		}
	} else {
		println!("WARNING: scancode queue uninitialized");
	}
}

pub struct ScancodeStream {
	_private: ()
}

impl ScancodeStream {
	pub fn new() -> Self {
		SCANCODE_QUEUE
			.try_init_once(|| ArrayQueue::new(100))
			.expect("ScancodeStream::new should only be called once");

		Self {
			_private: ()
		}
	}
}

impl Default for ScancodeStream {
	fn default() -> Self {
		Self::new()
	}
}

impl Stream for ScancodeStream {
	type Item = u8;

	fn poll_next(
		self: core::pin::Pin<&mut Self>,
		cx: &mut core::task::Context<'_>
	) -> core::task::Poll<Option<Self::Item>> {
		let queue = SCANCODE_QUEUE
			.try_get()
			.expect("SCANCODE_QUEUE not initialized");

		if let Some(scancode) = queue.pop() {
			return Poll::Ready(Some(scancode));
		}

		WAKER.register(cx.waker());

		match queue.pop() {
			Some(c) => {
				WAKER.take();
				Poll::Ready(Some(c))
			}
			None => Poll::Pending
		}
	}
}
