//! syscall.rs
//!
//! Syscall module for the kernel.
//!
//! Using custom system call commands as I would love this kernel to be unique
//! to me and others without resembling too much of UNIX/Linux

use alloc::vec::Vec;

use crate::{
	error::ERR_BAD_FD,
	fs::SCDirectoryEntryInfo,
	memory::user::{UserMutSlice, UserPtr, UserSlice, UserString},
	net::MsgHdr,
	process::args::argv_to_vec,
	serial_println,
	syscall::{
		closed::sys_closed,
		closef::sys_closef,
		//closesock::sys_closesock,
		connsock::sys_connsock,
		csocket::sys_csocket,
		getdirents::sys_getdirents,
		halt::sys_halt,
		//nap::sys_nap,
		opend::sys_opend,
		openf::sys_openf,
		readf::sys_readf,
		recv::sys_recv,
		rmdir::sys_rmdir,
		rmfile::sys_rmfile,
		run::sys_run,
		say::sys_say,
		send::sys_send,
		sizef::sys_sizef,
		spawnp::sys_spawnp,
		split::sys_split,
		stop::sys_stop,
		waiton::sys_waiton,
		writef::sys_writef
	},
	task::executor
};

pub mod closed;
pub mod closef;
pub mod closesock;
pub mod connsock;
pub mod csocket;
pub mod getdirents;
pub mod halt;
pub mod nap;
pub mod opend;
pub mod openf;
pub mod readf;
pub mod recv;
pub mod rmdir;
pub mod rmfile;
pub mod run;
pub mod say;
pub mod send;
pub mod sizef;
pub mod spawnp;

pub mod split;
pub mod stop;
pub mod waiton;
pub mod writef;

// syscall ids
const SYS_SAY: u32 = 0;
const SYS_HALT: u32 = 1;
const SYS_SPLIT: u32 = 2;
const SYS_WAITON: u32 = 3;
const SYS_OPENF: u32 = 4;
const SYS_CLOSEF: u32 = 5;
const SYS_READF: u32 = 6;
const SYS_WRITEF: u32 = 7;
const SYS_OPEND: u32 = 8;
const SYS_CLOSED: u32 = 9;
const SYS_RUN: u32 = 10;
const SYS_STOP: u32 = 11;
const SYS_NAP: u32 = 12;
const SYS_SIZEF: u32 = 13;
const SYS_CSOCKET: u32 = 14;
const SYS_CONNSOCK: u32 = 15;
const SYS_SEND: u32 = 16;
const SYS_RECV: u32 = 17;
const SYS_CLOSESOCK: u32 = 18;
const SYS_GETDIRENTS: u32 = 19;
const SYS_SPAWNP: u32 = 20;
const SYS_RMFILE: u32 = 21;
const SYS_RMDIR: u32 = 22;

/// Max length we're willing to pull in for a single user-provided path/string.
/// Prevents a malicious or buggy userspace program from claiming an enormous
/// length and forcing the kernel to allocate an unbounded buffer.
const MAX_USER_STRING_LEN: usize = 4096;

const O_RDONLY: u32 = 0x1;
const O_WRONLY: u32 = 0x2;
const O_RDWR: u32 = 0x4;
const O_CREAT: u32 = 0x8;

/// System call handler function. Called when the `syscall` or `int 0x80`
/// instruction is called.
///
/// # x86_64
/// Conforms to the conventional Linux-style syscall ABI:
/// - syscall_id in rax
/// - arg0 in rdi
/// - arg1 in rsi
/// - arg2 in rdx
/// - arg3 in r10
/// - arg4 in r8
/// - arg5 in r9
/// - return value in rax
///
/// # Safety
/// - Make sure valid arguments
pub unsafe fn syscall(
	syscall_id: u32,
	arg0: u64,
	arg1: u64,
	arg2: u64,
	arg3: u64,
	_arg4: u64,
	_arg5: u64
) -> i32 {
	let current_pid = unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			u64::MAX
		} else {
			(&(*executor::CURRENT_PROCESS_GUARD)).state.id.get()
		}
	};

	serial_println!("[SYSCALL] id={} pid={}", syscall_id, current_pid);

	match syscall_id {
		SYS_SAY => {
			let ptr = arg0 as *const u8;
			let len = arg1 as usize;
			serial_println!("SYS_SAY: ptr={:#?} len={}", ptr, len);
			let user_str = UserString {
				ptr,
				len
			};
			match user_str.read_to_kernel_string(MAX_USER_STRING_LEN) {
				Ok(s) => {
					serial_println!("SYS_SAY: kernel string len={}", s.len());
					sys_say(&s);
					0
				}
				Err(_) => {
					serial_println!("SYS_SAY: invalid user string");
					ERR_BAD_FD
				}
			}
		}
		SYS_HALT => {
			let exit_code = arg0 as i32;
			sys_halt(exit_code);
		}
		SYS_SPLIT => sys_split(),
		SYS_WAITON => sys_waiton(),
		SYS_OPENF => {
			let ptr = arg0 as *const u8;
			let len = arg1 as usize;
			let flags = arg2 as u32;
			let user_str = UserString {
				ptr,
				len
			};
			match user_str.read_to_kernel_string(MAX_USER_STRING_LEN) {
				Ok(path) => sys_openf(&path, flags),
				Err(_) => {
					serial_println!("SYS_OPENF: invalid user path");
					ERR_BAD_FD
				}
			}
		}
		SYS_CLOSEF => {
			let fd = arg0 as u32;
			sys_closef(fd)
		}
		SYS_READF => {
			let fd = arg0 as u32;
			let buf_ptr = arg1 as *mut u8;
			let len = arg2 as usize;
			let dst = UserMutSlice::<u8>::new(buf_ptr, len);
			sys_readf(fd, &dst)
		}
		SYS_WRITEF => {
			let fd = arg0 as u32;
			let buf_ptr = arg1 as *const u8;
			let len = arg2 as usize;
			let src = UserSlice::<u8> {
				ptr: buf_ptr,
				len
			};
			sys_writef(fd, &src)
		}
		SYS_OPEND => {
			let ptr = arg0 as *const u8;
			let len = arg1 as usize;
			let flags = arg2 as u32;
			let user_str = UserString {
				ptr,
				len
			};
			match user_str.read_to_kernel_string(MAX_USER_STRING_LEN) {
				Ok(path) => sys_opend(&path, flags),
				Err(_) => {
					serial_println!("SYS_OPEND: invalid user path");
					ERR_BAD_FD
				}
			}
		}
		SYS_CLOSED => {
			let fd = arg0 as u32;
			sys_closed(fd)
		}
		SYS_RUN => {
			let ptr = arg0 as *const u8;
			let len = arg1 as usize;
			let user_str = UserString {
				ptr,
				len
			};
			match user_str.read_to_kernel_string(MAX_USER_STRING_LEN) {
				Ok(path) => sys_run(&path),
				Err(_) => {
					serial_println!("SYS_RUN: invalid user path");
					ERR_BAD_FD
				}
			}
		}   
		SYS_STOP => {
    		let pid = arg0;
            if pid == 0 || pid == u64::MAX || pid == 0xFFFFFFFF {
                return -1;
            }
            sys_stop(arg0)
		},
		SYS_NAP => {
			serial_println!("i go nap nap now. sleep is a) broken, and b) unsafe :(");
			0
		}
		SYS_SIZEF => {
			let fd = arg0 as u32;
			sys_sizef(fd)
		}
		SYS_CSOCKET => sys_csocket(),
		SYS_CONNSOCK => {
			let fd = arg0 as u32;
			let host_ptr = arg1 as *const u8;
			let host_len = arg2 as usize;
			let port = arg3 as u64;

			let user_str = UserString {
				ptr: host_ptr,
				len: host_len
			};
			match user_str.read_to_kernel_string(MAX_USER_STRING_LEN) {
				Ok(host) => {
					serial_println!(
						"fd: {}\nhost_len: {}\nhost: {}\nport: {}",
						fd,
						host_len,
						host,
						port
					);
					sys_connsock(fd, &host, port)
				}
				Err(_) => {
					serial_println!("SYS_CONNSOCK: invalid user host string");
					ERR_BAD_FD
				}
			}
		}
		SYS_GETDIRENTS => {
			let fd = arg0 as u32;
			let out_ptr = arg1 as *mut SCDirectoryEntryInfo;
			let out_cap = arg2 as usize;
			let out = UserMutSlice::<SCDirectoryEntryInfo>::new(out_ptr, out_cap);
			sys_getdirents(fd, &out)
		}
		SYS_SPAWNP => {
			let path_ptr = arg0 as *const u8;
			let path_len = arg1 as usize;
			let user_str = UserString {
				ptr: path_ptr,
				len: path_len
			};

			let argc = arg2 as usize;
			let argv = arg3 as *const *const u8;

			match user_str.read_to_kernel_string(MAX_USER_STRING_LEN) {
				Ok(path) => {
					let args = match unsafe { argv_to_vec(argc, argv) } {
						Ok(a) => a,
						Err(_) => {
							serial_println!("spawnp: unable to convert argv to a vec.");
							return ERR_BAD_FD;
						}
					};
					sys_spawnp(&path, args, Vec::new())
				}
				Err(_) => {
					serial_println!("SYS_SPAWNP: invalid user path");
					ERR_BAD_FD
				}
			}
		}
		SYS_SEND => {
			let fd = arg0 as u32;
			let data_ptr = arg1 as *const u8;
			let data_len = arg2 as usize;
			let data = UserSlice::<u8> {
				ptr: data_ptr,
				len: data_len
			};
			sys_send(fd, &data)
		}
		SYS_RECV => {
			let fd = arg0 as u32;
			let msg_ptr = arg1 as *mut MsgHdr;
			let msg = UserPtr::<MsgHdr>::new(msg_ptr as *const MsgHdr);
			sys_recv(fd, &msg)
		}
		SYS_RMFILE => {
			let path_ptr = arg0 as *const u8;
			let path = UserSlice::<u8> {
				ptr: path_ptr,
				len: arg1 as usize
			};
			sys_rmfile(&path)
		}
		SYS_RMDIR => {
			let path_ptr = arg0 as *const u8;
			let path = UserSlice::<u8> {
				ptr: path_ptr,
				len: arg1 as usize
			};
			sys_rmdir(&path)
		}
		_ => {
			serial_println!("Invalid syscall ID: {}", syscall_id);
			-1
		}
	}
}

#[cfg(feature = "test")]
pub mod tests {
	use crate::{
		syscall::{
			MAX_USER_STRING_LEN,
			O_CREAT,
			O_RDONLY,
			O_RDWR,
			O_WRONLY,
			SYS_CLOSED,
			SYS_CLOSEF,
			SYS_CLOSESOCK,
			SYS_CONNSOCK,
			SYS_CSOCKET,
			SYS_GETDIRENTS,
			SYS_HALT,
			SYS_NAP,
			SYS_OPEND,
			SYS_OPENF,
			SYS_READF,
			SYS_RECV,
			SYS_RMDIR,
			SYS_RMFILE,
			SYS_RUN,
			SYS_SAY,
			SYS_SEND,
			SYS_SIZEF,
			SYS_SPAWNP,
			SYS_SPLIT,
			SYS_STOP,
			SYS_WAITON,
			SYS_WRITEF,
			syscall
		},
		tassert_eq,
		testing::ktest::TestError
	};

	pub fn test_max_user_string_len_value() -> Result<(), TestError> {
		tassert_eq!(MAX_USER_STRING_LEN, 4096);
		Ok(())
	}
	crate::create_test!(test_max_user_string_len_value);

	pub fn test_open_flags_values_and_combinations() -> Result<(), TestError> {
		tassert_eq!(O_RDONLY, 0x1);
		tassert_eq!(O_WRONLY, 0x2);
		tassert_eq!(O_RDWR, 0x4);
		tassert_eq!(O_CREAT, 0x8);

		tassert_eq!(O_RDONLY | O_CREAT, 0x9);
		tassert_eq!(O_WRONLY | O_CREAT, 0xA);
		tassert_eq!(O_RDWR | O_CREAT, 0xC);
		Ok(())
	}
	crate::create_test!(test_open_flags_values_and_combinations);

	pub fn test_syscall_ids_are_sequential_and_unique() -> Result<(), TestError> {
		tassert_eq!(SYS_SAY, 0);
		tassert_eq!(SYS_HALT, 1);
		tassert_eq!(SYS_SPLIT, 2);
		tassert_eq!(SYS_WAITON, 3);
		tassert_eq!(SYS_OPENF, 4);
		tassert_eq!(SYS_CLOSEF, 5);
		tassert_eq!(SYS_READF, 6);
		tassert_eq!(SYS_WRITEF, 7);
		tassert_eq!(SYS_OPEND, 8);
		tassert_eq!(SYS_CLOSED, 9);
		tassert_eq!(SYS_RUN, 10);
		tassert_eq!(SYS_STOP, 11);
		tassert_eq!(SYS_NAP, 12);
		tassert_eq!(SYS_SIZEF, 13);
		tassert_eq!(SYS_CSOCKET, 14);
		tassert_eq!(SYS_CONNSOCK, 15);
		tassert_eq!(SYS_SEND, 16);
		tassert_eq!(SYS_RECV, 17);
		tassert_eq!(SYS_CLOSESOCK, 18);
		tassert_eq!(SYS_GETDIRENTS, 19);
		tassert_eq!(SYS_SPAWNP, 20);
		tassert_eq!(SYS_RMFILE, 21);
		tassert_eq!(SYS_RMDIR, 22);
		Ok(())
	}
	crate::create_test!(test_syscall_ids_are_sequential_and_unique);

	pub fn test_syscall_invalid_id_returns_minus_one() -> Result<(), TestError> {
		let invalid_id = 9999;
		let result = unsafe { syscall(invalid_id, 0, 0, 0, 0, 0, 0) };
		tassert_eq!(result, -1);
		Ok(())
	}
	crate::create_test!(test_syscall_invalid_id_returns_minus_one);

	pub fn test_syscall_nap_returns_zero() -> Result<(), TestError> {
		let result = unsafe { syscall(SYS_NAP, 0, 0, 0, 0, 0, 0) };
		tassert_eq!(result, 0);
		Ok(())
	}
	crate::create_test!(test_syscall_nap_returns_zero);
}
