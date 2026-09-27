use alloc::{string::String, vec::Vec};
use core::sync::atomic::Ordering;

use crate::{
	error::{ERR_CRE_PROC, ERR_NO_ENT, ERR_NO_QUEUE},
	fs,
	process::util::spawn_user_process,
	serial_println,
	task::{FORCE_RESCHEDULE, executor::EXECUTOR}
};

pub fn sys_spawnp(path: &str, argv: Vec<String>, envs: Vec<String>) -> i32 {
	if !fs::with_fs(|fs| fs.exists(path)) {
		return ERR_NO_ENT;
	}

	let pid = fs::with_fs(|fs| match fs.read_file(path) {
		Ok(bytes) => match spawn_user_process(bytes, argv, envs) {
			Ok(process) => {
				let pid = process.state.id;
				{
					let mut executor = EXECUTOR.lock();
					if let Err(e) = executor.spawn_process(process) {
						serial_println!("sys_spawnp: failed to queue process: {}", e);
						return ERR_NO_QUEUE;
					}
					FORCE_RESCHEDULE.store(true, core::sync::atomic::Ordering::Relaxed);
				}
				pid.get() as i32
			}
			Err(e) => {
				serial_println!("sys_spawnp: failed to spawn new process: {}", e);
				ERR_CRE_PROC
			}
		},
		Err(_) => {
			serial_println!("sys_spawnp: file not found: {}", path);
			ERR_NO_ENT
		}
	});

	if pid > 0 {
		crate::task::FORCE_RESCHEDULE.store(true, Ordering::SeqCst);
	}

	pid
}
