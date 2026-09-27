use crate::task::{executor::EXECUTOR, process::ProcessId};

pub fn sys_stop(pid: u64) -> i32 {
	EXECUTOR.lock().end_process(ProcessId::new(pid), -2);
	0
}
