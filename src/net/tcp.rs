//!
//! net/tcp.rs
//!
//! TCP network handling.

use alloc::{boxed::Box, vec::Vec};
use core::{hint::spin_loop, net::Ipv4Addr};

use embedded_io::{ErrorType, Read, ReadReady, Write, WriteReady};
use smoltcp::{
	iface::SocketHandle,
	socket::tcp::{Socket, SocketBuffer},
	time::Instant,
	wire::{IpAddress, IpEndpoint}
};

use crate::{
	error::NullexError,
	net::{NET_MANAGER, http::httparse::url::Scheme},
	serial_println
};

const TCP_RX_BUFFER_SIZE: usize = 65_536;
const TCP_TX_BUFFER_SIZE: usize = 8192;
const TCP_RECV_CHUNK_SIZE: usize = 4096;
/// A structure representing a connection through the `TCP` protocol.
#[derive(Debug)]
pub struct TcpConnection {
	/// The handle for the TCP Connection.
	pub handle: SocketHandle,
	pub scheme: Scheme // todo: create a trait for these.
}

impl TcpConnection {
	/// Creates a new `TcpConnection`.
	pub fn new(scheme: Scheme) -> Self {
		let rx_buf_vec = Box::leak(Box::new(vec![0u8; TCP_RX_BUFFER_SIZE]));
		let tx_buf_vec = Box::leak(Box::new(vec![0u8; TCP_TX_BUFFER_SIZE]));
		let rx_buf = SocketBuffer::new(rx_buf_vec.as_mut_slice());
		let tx_buf = SocketBuffer::new(tx_buf_vec.as_mut_slice());
		let socket = Socket::new(rx_buf, tx_buf);
		let mut guard = NET_MANAGER.lock();
		let handle = guard.as_mut().unwrap().sockets.add(socket);
		Self {
			handle,
			scheme
		}
	}

	/// Connect to a specified IP address through the `TcpConnection`
	pub fn connect(
		&self,
		dst_ip: [u8; 4],
		dst_port: u16,
		src_port: u16
	) -> Result<(), NullexError> {
		let mut guard = NET_MANAGER.lock();
		let manager = guard.as_mut().ok_or(NullexError::NetworkNotInitialized)?;
		let remote = IpEndpoint::new(IpAddress::Ipv4(Ipv4Addr::from_octets(dst_ip)), dst_port);
		let socket = manager.sockets.get_mut::<Socket>(self.handle);
		socket
			.connect(manager.iface.context(), remote, src_port)
			.map_err(|e| {
				serial_println!("[TCP] Connect error: {:?}", e);
				NullexError::TcpConnectionFailed
			})?;

		core::mem::drop(guard);

		serial_println!("[TCP] Handshaking...");

		loop {
			let now = crate::rtc::rtc_instant();
			Self::poll(now);

			let mut guard = NET_MANAGER.lock();
			let manager = guard.as_mut().unwrap();
			let socket = manager.sockets.get::<Socket>(self.handle);
			let state = socket.state();
			core::mem::drop(guard);

			match state {
				smoltcp::socket::tcp::State::Established => {
					serial_println!("[TCP] Connected successfully!");
					break;
				}
				smoltcp::socket::tcp::State::Closed => {
					serial_println!("[TCP] Connection refused or timed out.");
					return Err(NullexError::TcpConnectionFailed);
				}
				_ => {
					core::hint::spin_loop();
				}
			}
		}

		Ok(())
	}

	/// If the `TcpConnection` is connected to a IP Address.
	pub fn is_connected(&self) -> bool {
		let mut guard = NET_MANAGER.lock();
		let manager = guard
			.as_mut()
			.ok_or(NullexError::NetworkNotInitialized)
			.expect("network not initialized.");
		let socket = manager.sockets.get::<Socket>(self.handle);

		matches!(
			socket.state(),
			smoltcp::socket::tcp::State::Established | smoltcp::socket::tcp::State::CloseWait
		)
	}

	/// Send data to the connected `TcpConnection`'s destination IP Address.
	pub fn send(&self, data: &[u8]) -> Result<usize, NullexError> {
		let mut guard = NET_MANAGER.lock();
		let manager = guard.as_mut().ok_or(NullexError::NetworkNotInitialized)?;
		let socket = manager.sockets.get_mut::<Socket>(self.handle);
		socket.send_slice(data).map_err(|e| {
			serial_println!("[TCP] Send Error: {:?}", e);
			NullexError::TcpFailedToSend
		})
	}

	/// Receive data from the connected `TcpConnection`'s destination IP
	/// address.
	pub fn recv(&self) -> Result<Vec<u8>, NullexError> {
		let mut chunk = [0u8; TCP_RECV_CHUNK_SIZE];
		let n = self.recv_into(&mut chunk)?;

		let mut out = Vec::new();
		if n > 0 {
			out.extend_from_slice(&chunk[..n]);
		}

		Ok(out)
	}

	/// Receive data from the connected `TcpConnection`'s destination IP
	/// address, and place it directly into a buffer.
	pub fn recv_into(&self, out: &mut [u8]) -> Result<usize, NullexError> {
		if out.is_empty() {
			return Ok(0);
		}

		let mut guard = NET_MANAGER.lock();
		let manager = guard
			.as_mut()
			.ok_or(NullexError::NetworkNotInitialized)
			.expect("network not initialized.");

		let socket = manager.sockets.get_mut::<Socket>(self.handle);
		if !socket.can_recv() {
			return Ok(0);
		}

		socket.recv_slice(out).map_err(|e| {
			serial_println!("[TCP] Recv Error: {:?}", e);
			NullexError::TcpFailedToReceive
		})
	}

	/// Close the `TcpConnection`
	pub fn close(&self) {
		let mut guard = NET_MANAGER.lock();
		let manager = guard
			.as_mut()
			.ok_or(NullexError::NetworkNotInitialized)
			.expect("network not initialized.");
		manager.sockets.get_mut::<Socket>(self.handle).close();
	}

	/// Poll the `TcpConnection`
	pub fn poll(timestamp: Instant) {
		let mut guard = NET_MANAGER.lock();
		let manager = guard
			.as_mut()
			.ok_or(NullexError::NetworkNotInitialized)
			.expect("network not initialized.");

		manager.poll(timestamp);
	}
}

impl ErrorType for TcpConnection {
	type Error = NullexError;
}

/// An `embedded-io` adapter for a smoltcp TCP connection.
pub struct TcpIo<'io, F>
where
	F: FnMut() -> Instant
{
	conn: &'io TcpConnection,
	now: F
}

impl<'io, F> TcpIo<'io, F>
where
	F: FnMut() -> Instant
{
	/// Creates a new blocking I/O adapter around an existing TCP connection.
	pub fn new(conn: &'io TcpConnection, now: F) -> Self {
		Self {
			conn,
			now
		}
	}

	/// Poll the `TcpIo`'s `TcpConnection`.
	pub fn pump(&mut self) {
		let now = (self.now)();
		TcpConnection::poll(now);
	}

	fn with_socket<R, G: FnOnce(&Socket) -> R>(&self, f: G) -> Result<R, NullexError> {
		let mut guard = NET_MANAGER.lock();
		let manager = guard.as_mut().ok_or(NullexError::NetworkNotInitialized)?;
		Ok(f(manager.sockets.get::<Socket>(self.conn.handle)))
	}
}

impl<'io, F> ErrorType for TcpIo<'io, F>
where
	F: FnMut() -> Instant
{
	type Error = NullexError;
}

impl<'io, F> ReadReady for TcpIo<'io, F>
where
	F: FnMut() -> Instant
{
	fn read_ready(&mut self) -> Result<bool, Self::Error> {
		self.pump();
		self.with_socket(|socket| socket.can_recv())
	}
}

impl<'io, F> WriteReady for TcpIo<'io, F>
where
	F: FnMut() -> Instant
{
	fn write_ready(&mut self) -> Result<bool, Self::Error> {
		self.pump();
		self.with_socket(|socket| socket.can_send())
	}
}

impl<'io, F> Read for TcpIo<'io, F>
where
	F: FnMut() -> Instant
{
	fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
		if buf.is_empty() {
			return Ok(0)
		}

		loop {
			self.pump();

			if self.with_socket(|socket| socket.can_recv())? {
				break;
			}

			if !self.with_socket(|socket| socket.is_active())? {
				return Ok(0);
			}

			spin_loop();
		}

		self.conn.recv_into(buf)
	}
}

impl<'io, F> Write for TcpIo<'io, F>
where
	F: FnMut() -> Instant
{
	fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
		if buf.is_empty() {
			return Ok(0)
		}

		loop {
			self.pump();

			if self.with_socket(|socket| socket.can_send())? {
				break;
			}

			if !self.with_socket(|socket| socket.is_active())? {
				return Ok(0);
			}

			spin_loop();
		}

		self.conn.send(buf)
	}

	fn flush(&mut self) -> Result<(), Self::Error> {
		loop {
			self.pump();

			if self.with_socket(|socket| socket.send_queue())? == 0 {
				return Ok(());
			}

			spin_loop();
		}
	}
}
