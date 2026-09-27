use alloc::boxed::Box;

use embedded_io::Write;
use embedded_tls::{Aes128GcmSha256, TlsConfig, TlsContext, UnsecureProvider, blocking};
use smoltcp::time::Instant;

use crate::{
	crypto::rng::KernelRng,
	error::NullexError,
	net::tcp::{TcpConnection, TcpIo},
	rtc::rtc_instant
};

pub const TLS_RECORD_BUFFER_SIZE: usize = 16640;

pub struct TlsSocket {
	pub tls:
		Option<blocking::TlsConnection<'static, TcpIo<'static, fn() -> Instant>, Aes128GcmSha256>>,
	pub conn: *mut TcpConnection,
	pub read_buf: *mut [u8],
	pub write_buf: *mut [u8]
}

impl core::fmt::Debug for TlsSocket {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("TlsSocket")
			.field("tls_present", &self.tls.is_some())
			.field("conn", &self.conn)
			.field("read_buf", &self.read_buf)
			.field("write_buf", &self.write_buf)
			.finish()
	}
}

impl TlsSocket {
	pub fn open(conn: TcpConnection, host: &str) -> Result<Self, NullexError> {
		let conn_ptr = Box::into_raw(Box::new(conn));
		let conn_ref: &'static TcpConnection = unsafe { &*conn_ptr };

		let read_buf = Box::into_raw(vec![0u8; TLS_RECORD_BUFFER_SIZE].into_boxed_slice());
		let write_buf = Box::into_raw(vec![0u8; TLS_RECORD_BUFFER_SIZE].into_boxed_slice());

		let read_slice: &'static mut [u8] = unsafe { &mut *read_buf };
		let write_slice: &'static mut [u8] = unsafe { &mut *write_buf };

		let transport = TcpIo::new(conn_ref, rtc_instant as fn() -> Instant);
		let rng = KernelRng::try_new().map_err(|_| NullexError::TlsFailed)?;
		let config = TlsConfig::new()
			.with_server_name(host)
			.enable_rsa_signatures();
		let provider = UnsecureProvider::new::<Aes128GcmSha256>(rng);

		let mut tls = blocking::TlsConnection::new(transport, read_slice, write_slice);
		if tls.open(TlsContext::new(&config, provider)).is_err() {
			drop(tls);
			unsafe {
				drop(Box::from_raw(read_buf));
				drop(Box::from_raw(write_buf));
				drop(Box::from_raw(conn_ptr));
			}

			return Err(NullexError::TlsFailed);
		}

		Ok(Self {
			tls: Some(tls),
			conn: conn_ptr,
			read_buf,
			write_buf
		})
	}

	pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, NullexError> {
		self.tls
			.as_mut()
			.ok_or(NullexError::TlsFailed)?
			.read(buf)
			.map_err(|_| NullexError::TlsFailed)
	}

	pub fn write_all(&mut self, buf: &[u8]) -> Result<(), NullexError> {
		let tls = self.tls.as_mut().ok_or(NullexError::TlsFailed)?;
		tls.write_all(buf).map_err(|_| NullexError::TlsFailed)?;
		tls.flush().map_err(|_| NullexError::TlsFailed)
	}

	pub fn connection(&self) -> &TcpConnection {
		unsafe { &*self.conn }
	}

	pub fn close(&mut self) {
		drop(self.tls.take());
		unsafe { &mut *self.conn }.close();
	}
}

impl Drop for TlsSocket {
	fn drop(&mut self) {
		drop(self.tls.take());

		unsafe {
			drop(Box::from_raw(self.read_buf));
			drop(Box::from_raw(self.write_buf));
			drop(Box::from_raw(self.conn));
		}
	}
}

#[derive(Debug)]
pub enum SocketTransport {
	Plain(TcpConnection),
	Tls(TlsSocket)
}

impl SocketTransport {
	pub fn is_connected(&self) -> bool {
		match self {
			SocketTransport::Plain(conn) => conn.is_connected(),
			SocketTransport::Tls(tls) => tls.connection().is_connected()
		}
	}
}
