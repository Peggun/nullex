use alloc::vec::Vec;
use smoltcp::{iface::{Interface, SocketSet}, socket::tcp::Socket, time::Instant};

use crate::{drivers::virtio::net::VirtioNet, error::NullexError, net::tcp::TcpConnection, serial_println};

pub struct HttpResponse {
    pub status_code: u16,
    pub body: Vec<u8>,
}

pub fn http_get(iface: &mut Interface, device: &mut VirtioNet, sockets: &mut SocketSet<'_>, dst_ip: [u8; 4], dst_port: u16, host: &str, path: &str, src_port: u16, now: Instant) -> Result<HttpResponse, NullexError> {
    let conn = TcpConnection::new(sockets);
    conn.connect(iface, sockets, dst_ip, dst_port, src_port)?;
    serial_println!("[HTTP] Connecting to {}:{}", host, dst_port);

    let mut timestamp = now;
    let mut ticks = 0u64;
    loop {
        TcpConnection::poll(iface, device, sockets, timestamp);

        let state = sockets.get::<Socket>(conn.handle).state();
        serial_println!("[HTTP] TCP state: {:?}", state);

        match state {
            smoltcp::socket::tcp::State::Established => break,
            smoltcp::socket::tcp::State::Closed
            | smoltcp::socket::tcp::State::TimeWait => {
                return Err(NullexError::TcpConnectionFailed);
            }
            _ => {}
        }

        ticks += 1;
        if ticks > 10000 {
            serial_println!("[HTTP] Connect timed out");
            return Err(NullexError::TcpConnectionFailed);
        }

        timestamp = Instant::from_millis(timestamp.millis() + 1);
    }
    serial_println!("[HTTP] Connected.");

    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: */*\r\n\r\n",
        path, host
    );
    conn.send(sockets, request.as_bytes())?;
    serial_println!("[HTTP] Request sent. ({} bytes)", request.len());

    let mut raw_response = Vec::new();
    let mut stall_ticks = 0u64;

    loop {
        TcpConnection::poll(iface, device, sockets, timestamp);

        let chunk = conn.recv(sockets)?;
        if !chunk.is_empty() {
            serial_println!("[HTTP] Received {} bytes", chunk.len());
            raw_response.extend_from_slice(&chunk);
            stall_ticks = 0;
        } else {
            stall_ticks += 1;
        }

        let finished = {
            let socket = sockets.get::<Socket>(conn.handle);
            !socket.is_active()
        };

        if finished {
            serial_println!("[HTTP] Server closed connection");
            break;
        }

        if stall_ticks > 5000 {
            serial_println!("[HTTP] Stalled waiting for data, giving up");
            break;
        }

        timestamp = Instant::from_millis(timestamp.millis() + 1);
    }

    conn.close(sockets);
    serial_println!("[HTTP] Connection closed, {} bytes total", raw_response.len());

    parse_response(raw_response)
}

fn parse_response(raw: Vec<u8>) -> Result<HttpResponse, NullexError> {
    let split = raw.windows(4).position(|w| w == b"\r\n\r\n").ok_or(NullexError::HttpInvalidResponse)?;

    let header_section = str::from_utf8(&raw[..split]).map_err(|_| NullexError::HttpInvalidResponse)?;

    let status_code = header_section
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or(NullexError::HttpInvalidResponse)?;

    let body = raw[split + 4..].to_vec();
    serial_println!("[HTTP] Status: {}, body: {} bytes", status_code, body.len());

    Ok(HttpResponse { status_code, body })
}