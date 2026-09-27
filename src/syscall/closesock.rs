use crate::{
	error::{ERR_BAD_FD, ERR_NOT_CONNECTED},
	net::socket::SocketTransport,
	serial_println,
	task::{executor, files::FileBackend}
};

pub fn closesock(fd: u32) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_closesock: no current process guard.");
			return -1;
		}

		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		let file = match process.open_files.get_mut(&fd) {
			Some(file) => file,
			None => {
				serial_println!("sys_closesock: invalid fd: {}", fd);
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

		match transport {
			SocketTransport::Plain(conn) => {
				conn.close();
				return 0;
			}
			SocketTransport::Tls(tls) => {
				tls.close();
				return 0;
			}
		}
	}
}
