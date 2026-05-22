//!
//! net/tcp.rs
//! 
//! TCP network handling.
//! 

use alloc::{boxed::Box, vec::Vec};
use core::net::Ipv4Addr;

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

	/// Receive data from the connected `TcpConnection`'s destination IP Address.
	pub fn recv(&self, sockets: &mut SocketSet<'_>) -> Result<Vec<u8>, NullexError> {
		let mut chunk = [0u8; TCP_RECV_CHUNK_SIZE];
		let n = self.recv_into(sockets, &mut chunk)?;

		let mut out = Vec::new();
		if n > 0 {
			out.extend_from_slice(&chunk[..n]);
		}

		Ok(out)
	}

	/// Receive data from the connected `TcpConnection`'s destination IP address, and place it directly into a buffer.
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
