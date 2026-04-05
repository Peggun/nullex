use core::net::Ipv4Addr;

use alloc::{boxed::Box, vec::Vec};
use smoltcp::{iface::{Interface, SocketHandle, SocketSet}, socket::tcp::{self, Socket, SocketBuffer}, time::Instant, wire::{IpAddress, IpEndpoint}};

use crate::{drivers::virtio::net::VirtioNet, error::NullexError, serial_println};

const TCP_RX_BUFFER_SIZE: usize = 8192;
const TCP_TX_BUFFER_SIZE: usize = 8192;

pub struct TcpConnection {
    pub handle: SocketHandle
}

impl TcpConnection  {
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

    pub fn connect(&self, iface: &mut Interface, sockets: &mut SocketSet<'_>, dst_ip: [u8; 4], dst_port: u16, src_port: u16) -> Result<(), NullexError> {
        let remote = IpEndpoint::new(
            IpAddress::Ipv4(Ipv4Addr::from_octets(dst_ip)),
            dst_port
        );

        let socket = sockets.get_mut::<Socket>(self.handle);
        socket.connect(iface.context(), remote, src_port)
            .map_err(|e| {
                serial_println!("[TCP] Connect error: {:?}", e);
                NullexError::TcpConnectionFailed
            })
    }

    pub fn is_connected(&self, sockets: &mut SocketSet<'_>) -> bool {
        sockets.get::<Socket>(self.handle).is_active()
    }

    pub fn send(&self, sockets: &mut SocketSet<'_>, data: &[u8]) -> Result<usize, NullexError> {
        let socket = sockets.get_mut::<Socket>(self.handle);
        socket.send_slice(data)
            .map_err(|e| {
                serial_println!("[TCP] Send Error: {:?}", e);
                NullexError::TcpFailedToSend
            })

    }

    pub fn recv(&self, sockets: &mut SocketSet<'_>) -> Result<Vec<u8>, NullexError> {
        let socket = sockets.get_mut::<Socket>(self.handle);
        if !socket.can_recv() {
            return Ok(vec![]);
        }

        let mut buf = vec![0u8; TCP_RX_BUFFER_SIZE];
        let n = socket.recv_slice(&mut buf).map_err(|e| {
            serial_println!("[TCP] Recv Error: {:?}", e);
            NullexError::TcpFailedToReceive
        })?;
        buf.truncate(n);
        Ok(buf)
    }

    pub fn close(&self, sockets: &mut SocketSet<'_>) {
        sockets.get_mut::<Socket>(self.handle).close();
    }

    pub fn poll(iface: &mut Interface, device: &mut VirtioNet, sockets: &mut SocketSet<'_>, timestamp: Instant) {
        iface.poll(timestamp, device, sockets);
    }
}