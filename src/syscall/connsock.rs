use crate::{
	error::{
		ERR_BAD_FD,
		ERR_BAD_URL,
		ERR_DNS_FAIL,
		ERR_GATEWAY_UNREACH,
		ERR_TCP_CONN_FAIL,
		ERR_TLS_FAILED
	},
	net::{
		GATEWAY_IP,
		http::{
			httparse::url::{ParsedUrl, Scheme},
			next_src_port
		},
		socket::{SocketTransport, TlsSocket},
		tcp::TcpConnection
	},
	println,
	serial_println,
	task::{executor, files::FileBackend, yield_now}
};

// CURRENTLY ONLY SUPPORTS HTTP(S)
pub fn sys_connsock(fd: u32, host: &str, port: u64) -> i32 {
	unsafe {
		if executor::CURRENT_PROCESS_GUARD.is_null() {
			serial_println!("sys_connsock: no current process guard");
			return -1;
		}

		let process = &mut *executor::CURRENT_PROCESS_GUARD;
		if process.open_files.get(&fd).is_none() {
			serial_println!("sys_connsock: invalid fd: {}", fd);
			return ERR_BAD_FD;
		} else if !matches!(
			process.open_files.get_mut(&fd).unwrap().backend,
			FileBackend::Socket { .. }
		) {
			serial_println!("sys_connsock: fd is not a socket: {}", fd);
			return ERR_BAD_FD;
		}

		let url = match ParsedUrl::parse(host) {
			Ok(pu) => pu,
			Err(e) => {
				serial_println!("invalid url with error: {:?}", e);
				return ERR_BAD_URL;
			}
		};

		let dst_port = if port != 0 {
			if port > u16::MAX as u64 {
				serial_println!("sys_connsock: invalid port: {}", port);
				return ERR_BAD_URL;
			}
			port as u16
		} else {
			url.port
		};

		let dst_ip = match crate::net::dns::resolve(&url.host) {
			Ok(dst_ip) => dst_ip,
			Err(e) => {
				serial_println!("sys_dnsrslv: DNS failed: {:?}", e);
				return ERR_DNS_FAIL;
			}
		};

		if crate::net::arp::get_cached(GATEWAY_IP).is_none() {
			serial_println!("[NGET] Resolving gateway MAC before TCP connect...");
			if let Err(e) = crate::net::send_arp_request(GATEWAY_IP)
				.and_then(|_| crate::net::arp::wait_for_arp(GATEWAY_IP, 2000).map(|_| ()))
			{
				println!("nget: gateway ARP failed: {:?}", e);
				return ERR_GATEWAY_UNREACH;
			}
		}

		let conn_result = {
			let _kg = crate::task::address_space::Cr3Guard::enter_kernel();

			let src_port = next_src_port();
			let conn = TcpConnection::new(url.scheme);
			let r = conn.connect(dst_ip, dst_port, src_port);

			if r.is_ok() {
				while !conn.is_connected() {
					yield_now();
				}

				let transport = match url.scheme {
					Scheme::Https => match TlsSocket::open(conn, &url.host) {
						Ok(tls) => SocketTransport::Tls(tls),
						Err(e) => {
							serial_println!("sys_connsock: TLS handshake failed: {:?}", e);
							return ERR_TLS_FAILED;
						}
					},
					Scheme::Http => SocketTransport::Plain(conn)
				};

				let file_backend = process.open_files.get_mut(&fd).unwrap();
				if let FileBackend::Socket {
					connection
				} = &mut file_backend.backend
				{
					*connection = Some(transport);
				}
			}

			r
		};

		if conn_result.is_ok() {
			return 0;
		} else {
			return ERR_TCP_CONN_FAIL;
		}
	}
}
