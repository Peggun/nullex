use crate::{
	error::{ERR_BAD_FD, ERR_FAILED_TO_SEND, ERR_NOT_CONNECTED},
	memory::user::UserSlice,
	net::socket::SocketTransport,
	serial_println,
	task::{executor, files::FileBackend}
};

// CURRENTLY ONLY SUPPORTS HTTP(S)
pub fn sys_send(fd: u32, src: &UserSlice<u8>) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_send: no current process guard");
			return -1;
		}

		let process = &mut *executor::CURRENT_PROCESS_GUARD;

		let file = match process.open_files.get_mut(&fd) {
			Some(file) => file,
			None => {
				serial_println!("sys_send: invalid fd: {}", fd);
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
				serial_println!("sys_send: socket {} has no connection", fd);
				return ERR_NOT_CONNECTED;
			}
			_ => {
				serial_println!("sys_send: fd is not a socket: {}", fd);
				return ERR_BAD_FD;
			}
		};

		if !transport.is_connected() {
			serial_println!("sys_send: socket {} is not connected", fd);
			return ERR_NOT_CONNECTED;
		}

		let mut kernel_buf = vec![0u8; src.len];
		if let Err(_) = src.copy_to_kernel(&mut kernel_buf) {
			serial_println!("sys_send: failed to copy from user buffer");
			return ERR_BAD_FD;
		}

		match transport {
			SocketTransport::Plain(conn) => match conn.send(&kernel_buf) {
				Ok(bytes_written) => bytes_written as i32,
				Err(e) => {
					serial_println!("sys_send: failed to send: {:?}", e);
					ERR_FAILED_TO_SEND
				}
			},
			SocketTransport::Tls(tls) => match tls.write_all(&kernel_buf) {
				Ok(()) => kernel_buf.len() as i32,
				Err(e) => {
					serial_println!("sys_send: failed to send over tls: {:?}", e);
					ERR_FAILED_TO_SEND
				}
			}
		}
	}
}
