use alloc::sync::Arc;
use core::sync::atomic::AtomicBool;

use futures::task::AtomicWaker;

use crate::{
	serial_println,
	sync::oncecell::cell::OnceCell,
	task::{
		executor::{CURRENT_PROCESS, EXECUTOR},
		process::{Process, ProcessState}
	}
};

pub fn sys_split() -> i32 {
	serial_println!("sys_split called");
	let current_state = {
		let locked = CURRENT_PROCESS.lock();
		locked
			.as_ref()
			.expect("No current process during sys_split")
			.clone()
	};
	let future_fn_clone = current_state.future_fn.clone();
	let mut executor = EXECUTOR.lock();
	let child_pid = executor.create_pid();
	let child_state = Arc::new(ProcessState {
		id: child_pid,
		is_child: true,
		future_fn: future_fn_clone,
		queued: AtomicBool::new(false),
		scancode_queue: OnceCell::uninit(),
		waker: AtomicWaker::new()
	});
	let child_process = Process::new(child_state).expect("Process created incorrectly.");
	match executor.spawn_process(child_process) {
		Ok(()) => child_pid.get() as i32,
		Err(_) => -1
	}
}
