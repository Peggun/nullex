use alloc::vec::Vec;
use core::hint::spin_loop;

use crate::{
	error::{ERR_BAD_FD, ERR_NOT_CONNECTED, ERR_TIMED_OUT},
	memory::user::{UserMutSlice, UserPtr, UserSlice},
	net::{
		IoVec,
		MsgHdr,
		MsgType,
		http::{HTTP_RECV_CHUNK_SIZE, RESPONSE_STALL_TIMEOUT_MS, socket_finished},
		socket::SocketTransport,
		tcp::TcpConnection
	},
	serial_println,
	task::{executor, files::FileBackend},
	time::elapsed_ms
};

// CURRENTLY ONLY SUPPORTS HTTP(S)
pub fn sys_recv(fd: u32, user_msg: &UserPtr<MsgHdr>) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_recv: no current process guard");
			return -1;
		}

		let process = &mut *executor::CURRENT_PROCESS_GUARD;

		let file = match process.open_files.get_mut(&fd) {
			Some(file) => file,
			None => {
				serial_println!("sys_recv: invalid fd: {}", fd);
				return ERR_BAD_FD;
			}
		};

		let transport = match &mut file.backend {
			FileBackend::Socket {
				connection: Some(transport)
			} => transport,

			FileBackend::Socket {
				connection: None
			} => {
				serial_println!("sys_recv: socket {} has no connection", fd);
				return ERR_NOT_CONNECTED;
			}

			_ => {
				serial_println!("sys_recv: fd is not a socket: {}", fd);
				return ERR_BAD_FD;
			}
		};

		if !transport.is_connected() {
			serial_println!("sys_recv: socket {} is not connected", fd);
			return ERR_NOT_CONNECTED;
		}

		let msg = match user_msg.read() {
			Ok(msg) => msg,
			Err(_) => {
				serial_println!("sys_recv: msg pointer is invalid");
				return ERR_BAD_FD;
			}
		};

		let mut current_vec_idx = 0;
		let mut total_bytes_written_to_user = 0;

		// build the list of IoVec's
		let io_vectors: Vec<UserMutSlice<u8>> = match msg.msg_type {
			MsgType::Raw => {
				let raw = msg.payload.raw;

				if raw.len > 0 && raw.buf.is_null() {
					serial_println!("sys_recv: raw payload buffer is null");
					return ERR_BAD_FD;
				}

				alloc::vec![UserMutSlice::<u8>::new(raw.buf, raw.len)]
			}
			MsgType::Vec => {
				let vec = msg.payload.vec;

				if vec.msg_iovcnt == 0 {
					return 0;
				}

				if vec.msg_iov.is_null() {
					serial_println!("sys_recv: iovec pointer is null");
					return ERR_BAD_FD;
				}

				if (vec.msg_iov as usize) % core::mem::align_of::<IoVec>() != 0 {
					serial_println!("sys_recv: iovec pointer is not aligned");
					return ERR_BAD_FD;
				}

				let iovec_src = UserSlice::<IoVec> {
					ptr: vec.msg_iov,
					len: vec.msg_iovcnt
				};

				let mut iovec_buf = vec![
					IoVec {
						iov_base: core::ptr::null_mut(),
						iov_len: 0
					};
					vec.msg_iovcnt
				];

				if iovec_src.copy_to_kernel(&mut iovec_buf).is_err() {
					serial_println!("sys_recv: failed to copy iovec array");
					return ERR_BAD_FD;
				}

				iovec_buf
					.into_iter()
					.map(|iov| UserMutSlice::new(iov.iov_base, iov.iov_len))
					.collect()
			}
		};

		if io_vectors.is_empty() {
			return 0;
		}

		let mut current_vec = &io_vectors[current_vec_idx];
		let mut current_offset = 0;

		let mut timestamp = crate::rtc::rtc_instant();
		let mut last_progress = timestamp;
		let mut recv_buf = [0u8; HTTP_RECV_CHUNK_SIZE];

		loop {
			timestamp = crate::rtc::rtc_instant();

			let read = match transport {
				SocketTransport::Plain(conn) => {
					TcpConnection::poll(timestamp);
					match conn.recv_into(&mut recv_buf) {
						Ok(read) => read,
						Err(_) => {
							if total_bytes_written_to_user > 0 {
								match transport {
									SocketTransport::Plain(c) => c.close(),
									SocketTransport::Tls(t) => t.close()
								}
								return total_bytes_written_to_user;
							}
							return -1;
						}
					}
				}
				SocketTransport::Tls(tls) => match tls.read(&mut recv_buf) {
					Ok(read) => read,
					Err(_) => {
						if total_bytes_written_to_user > 0 {
							match transport {
								SocketTransport::Plain(c) => c.close(),
								SocketTransport::Tls(t) => t.close()
							}
							return total_bytes_written_to_user;
						}
						return -1;
					}
				}
			};

			if read > 0 {
				last_progress = timestamp;

				let chunk = &recv_buf[..read];
				let mut chunk_offset = 0;

				while chunk_offset < chunk.len() {
					if current_offset >= current_vec.len {
						current_vec_idx += 1;

						if current_vec_idx >= io_vectors.len() {
							return total_bytes_written_to_user;
						}

						current_vec = &io_vectors[current_vec_idx];
						current_offset = 0;
					}

					let available = current_vec.len - current_offset;
					let remaining = chunk.len() - chunk_offset;
					let to_copy = core::cmp::min(available, remaining);

					let piece = &chunk[chunk_offset..chunk_offset + to_copy];

					let dst = UserMutSlice::<u8>::new(current_vec.ptr.add(current_offset), to_copy);

					match dst.copy_from_kernel(piece) {
						Ok(written) => {
							current_offset += written;
							chunk_offset += written;
							total_bytes_written_to_user += written as i32;

							serial_println!("written = {}", written);

							if written != to_copy {
								return total_bytes_written_to_user;
							}
						}
						Err(_) => {
							serial_println!("sys_recv: failed to copy chunk");
							return total_bytes_written_to_user;
						}
					}
				}

				continue;
			}

			let finished = match transport {
				SocketTransport::Plain(conn) => socket_finished(conn),
				SocketTransport::Tls(tls) => socket_finished(tls.connection())
			};

			if finished {
				match transport {
					SocketTransport::Plain(conn) => conn.close(),
					SocketTransport::Tls(tls) => tls.close()
				}

				return total_bytes_written_to_user;
			}

			if elapsed_ms(last_progress, timestamp) >= RESPONSE_STALL_TIMEOUT_MS {
				match transport {
					SocketTransport::Plain(conn) => conn.close(),
					SocketTransport::Tls(tls) => tls.close()
				}

				return ERR_TIMED_OUT;
			}

			spin_loop();
		}
	}
}
