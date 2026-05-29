//!
//! net/tcp.rs
//!
//! TCP network handling.

use alloc::{boxed::Box, vec::Vec};
use core::{hint::spin_loop, net::Ipv4Addr};

use embedded_io::{ErrorType, Read, ReadReady, Write, WriteReady};
use smoltcp::{
	iface::{Interface, SocketHandle, SocketSet},
	socket::tcp::{Socket, SocketBuffer},
	time::Instant,
	wire::{IpAddress, IpEndpoint}
};

use crate::{drivers::virtio::net::VirtioNet, error::NullexError, serial_println};

const TCP_RX_BUFFER_SIZE: usize = 65_536;
const TCP_TX_BUFFER_SIZE: usize = 8192;
const TCP_RECV_CHUNK_SIZE: usize = 4096;

/// A structure representing a connection through the `TCP` protocol.
pub struct TcpConnection {
	/// The handle for the TCP Connection.
	pub handle: SocketHandle
}

impl TcpConnection {
	/// Creates a new `TcpConnection` within the specified socket set.
	pub fn new(sockets: &mut SocketSet<'_>) -> Self {
		let rx_buf_vec = Box::leak(Box::new(vec![0u8; TCP_RX_BUFFER_SIZE]));
		let tx_buf_vec = Box::leak(Box::new(vec![0u8; TCP_TX_BUFFER_SIZE]));
		let rx_buf = SocketBuffer::new(rx_buf_vec.as_mut_slice());
		let tx_buf = SocketBuffer::new(tx_buf_vec.as_mut_slice());
		let socket = Socket::new(rx_buf, tx_buf);
		let handle = sockets.add(socket);
		Self {
			handle
		}
	}

	/// Connect to a specified IP address through the `TcpConnection`
	pub fn connect(
		&self,
		iface: &mut Interface,
		sockets: &mut SocketSet<'_>,
		dst_ip: [u8; 4],
		dst_port: u16,
		src_port: u16
	) -> Result<(), NullexError> {
		let remote = IpEndpoint::new(IpAddress::Ipv4(Ipv4Addr::from_octets(dst_ip)), dst_port);
		let socket = sockets.get_mut::<Socket>(self.handle);
		socket
			.connect(iface.context(), remote, src_port)
			.map_err(|e| {
				serial_println!("[TCP] Connect error: {:?}", e);
				NullexError::TcpConnectionFailed
			})
	}

	/// If the `TcpConnection` is connected to a IP Address.
	pub fn is_connected(&self, sockets: &mut SocketSet<'_>) -> bool {
		sockets.get::<Socket>(self.handle).is_active()
	}

	/// Send data to the connected `TcpConnection`'s destination IP Address.
	pub fn send(&self, sockets: &mut SocketSet<'_>, data: &[u8]) -> Result<usize, NullexError> {
		let socket = sockets.get_mut::<Socket>(self.handle);
		socket.send_slice(data).map_err(|e| {
			serial_println!("[TCP] Send Error: {:?}", e);
			NullexError::TcpFailedToSend
		})
	}

	/// Receive data from the connected `TcpConnection`'s destination IP
	/// Address.
	pub fn recv(&self, sockets: &mut SocketSet<'_>) -> Result<Vec<u8>, NullexError> {
		let mut chunk = [0u8; TCP_RECV_CHUNK_SIZE];
		let n = self.recv_into(sockets, &mut chunk)?;

		let mut out = Vec::new();
		if n > 0 {
			out.extend_from_slice(&chunk[..n]);
		}

		Ok(out)
	}

	/// Receive data from the connected `TcpConnection`'s destination IP
	/// address, and place it directly into a buffer.
	pub fn recv_into(
		&self,
		sockets: &mut SocketSet<'_>,
		out: &mut [u8]
	) -> Result<usize, NullexError> {
		if out.is_empty() {
			return Ok(0);
		}

		let socket = sockets.get_mut::<Socket>(self.handle);
		if !socket.can_recv() {
			return Ok(0);
		}

		socket.recv_slice(out).map_err(|e| {
			serial_println!("[TCP] Recv Error: {:?}", e);
			NullexError::TcpFailedToReceive
		})
	}

	/// Close the `TcpConnection`
	pub fn close(&self, sockets: &mut SocketSet<'_>) {
		sockets.get_mut::<Socket>(self.handle).close();
	}

	/// Poll the `TcpConnection`
	pub fn poll(
		iface: &mut Interface,
		device: &mut VirtioNet,
		sockets: &mut SocketSet<'_>,
		timestamp: Instant
	) {
		iface.poll(timestamp, device, sockets);
	}
}

impl ErrorType for TcpConnection {
	type Error = NullexError;
}

/// An `embedded-io` adapter for a smoltcp TCP connection.
pub struct TcpIo<'io, 's, F>
where
	F: FnMut() -> Instant
{
	conn: &'io TcpConnection,
	iface: &'io mut Interface,
	device: &'io mut VirtioNet,
	sockets: &'io mut SocketSet<'s>,
	now: F
}

impl<'io, 's, F> TcpIo<'io, 's, F>
where
	F: FnMut() -> Instant
{
	/// Creates a new blocking I/O adapter around an existing TCP connection.
	pub fn new(
		conn: &'io TcpConnection,
		iface: &'io mut Interface,
		device: &'io mut VirtioNet,
		sockets: &'io mut SocketSet<'s>,
		now: F
	) -> Self {
		Self {
			conn,
			iface,
			device,
			sockets,
			now
		}
	}

	/// Poll the `TcpIo`'s `TcpConnection`.
	pub fn pump(&mut self) {
		let now = (self.now)();
		TcpConnection::poll(self.iface, self.device, self.sockets, now);
	}

	fn socket(&self) -> &Socket<'s> {
		self.sockets.get::<Socket>(self.conn.handle)
	}
}

impl<'io, 's, F> ErrorType for TcpIo<'io, 's, F>
where
	F: FnMut() -> Instant
{
	type Error = NullexError;
}

impl<'io, 's, F> ReadReady for TcpIo<'io, 's, F>
where
	F: FnMut() -> Instant
{
	fn read_ready(&mut self) -> Result<bool, Self::Error> {
		self.pump();
		Ok(self.socket().can_recv())
	}
}

impl<'io, 's, F> WriteReady for TcpIo<'io, 's, F>
where
	F: FnMut() -> Instant
{
	fn write_ready(&mut self) -> Result<bool, Self::Error> {
		self.pump();
		Ok(self.socket().can_send())
	}
}

impl<'io, 's, F> Read for TcpIo<'io, 's, F>
where
	F: FnMut() -> Instant
{
	fn read(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
		if buf.is_empty() {
			return Ok(0)
		}

		loop {
			self.pump();

			let s = self.socket();
			if s.can_recv() {
				break;
			}

			if !s.is_active() {
				return Ok(0);
			}

			spin_loop();
		}

		self.conn.recv_into(self.sockets, buf)
	}
}

impl<'io, 's, F> Write for TcpIo<'io, 's, F>
where
	F: FnMut() -> Instant
{
	fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
		if buf.is_empty() {
			return Ok(0)
		}

		loop {
			self.pump();

			let s = self.socket();
			if s.can_send() {
				break;
			}

			if !s.is_active() {
				return Ok(0);
			}

			spin_loop();
		}

		self.conn.send(self.sockets, buf)
	}

	fn flush(&mut self) -> Result<(), Self::Error> {
		loop {
			self.pump();

			if self.socket().send_queue() == 0 {
				return Ok(());
			}

			spin_loop();
		}
	}
}
