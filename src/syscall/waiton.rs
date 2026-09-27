use crate::{executor, serial_println};

pub fn sys_waiton() -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_wait: No current process guard");
			return -1;
		}
		let _process = &mut *executor::CURRENT_PROCESS_GUARD;
		0
	}
}
