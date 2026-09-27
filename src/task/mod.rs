//!
//! src/task/mod.rs
//!
//! Module definition for the task handling for the kernel.

pub mod address_space;
pub mod context;
pub mod cr3;
pub mod executor;
pub mod files;
pub mod keyboard;
pub mod process;

use core::{
	future::Future,
	pin::Pin,
	sync::atomic::{AtomicBool, Ordering},
	task::{Context, Poll}
};

use crate::arch::x86_64::user::{
	KERNEL_CR3,
	USER_EXIT_CODE,
	USER_EXIT_REQUESTED,
	enter_user_process
};

const KERNEL_STACK_SIZE: usize = 4096 * 32;
const KERNEL_STACK_PAGES_TO_MAP: usize = KERNEL_STACK_SIZE / 4096;

pub static FORCE_RESCHEDULE: AtomicBool = AtomicBool::new(false);

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

pub struct UserProcessFuture;

impl Future for UserProcessFuture {
	type Output = i32;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		unsafe {
			if executor::CURRENT_PROCESS_GUARD.is_null() {
				return Poll::Ready(-1);
			}

			enter_user_process(&*executor::CURRENT_PROCESS_GUARD);
		}

		if USER_EXIT_REQUESTED.load(Ordering::SeqCst) {
			USER_EXIT_REQUESTED.store(false, Ordering::SeqCst);
			Poll::Ready(USER_EXIT_CODE.load(Ordering::SeqCst))
		} else {
			cx.waker().wake_by_ref();
			Poll::Pending
		}
	}
}
