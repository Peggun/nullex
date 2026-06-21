//!
//! net/http.rs
//!
//! HTTP network request handling.

use alloc::{string::String, vec::Vec};
use core::{
	hint::spin_loop,
	sync::atomic::{AtomicU16, Ordering}
};

use smoltcp::{
	iface::{Interface, SocketSet},
	socket::tcp::Socket,
	time::Instant
};

use crate::{
	drivers::virtio::net::VirtioNet,
	error::NullexError,
	net::{dns::resolve, https::do_https_fetch_once, tcp::TcpConnection},
	serial_println,
	utils::{
		httparse::{
			chunked::decode_chunked,
			headers::ResponseHeaders,
			response::{HttpResult, ResponseKind, classify, resolve_filename},
			url::{ParsedUrl, Scheme},
			writer::{DownloadedFileWriter, FileSystemDownloadedFileWriter}
		},
		time::elapsed_ms
	}
};

/// Static reference to the next ephemeral port.
pub static NEXT_EPHEMERAL_PORT: AtomicU16 = AtomicU16::new(49152);

/// Http Response
pub struct HttpResponse {
	/// Status code
	pub status_code: u16,
	/// Body
	pub body: Vec<u8>
}

/// Enum stating the current stage a fetch is in.
pub enum FetchStep {
	/// The fetch has been completed
	Complete(HttpResult),
	/// The fetch resolves to a redirect
	Redirect(String)
}

/// HTTP(S) receive chunk size.
pub const HTTP_RECV_CHUNK_SIZE: usize = 4096;
/// HTTP(S) connection timeout.
pub const CONNECT_TIMEOUT_MS: i64 = 10_000;
/// HTTP(S) connection log interval.
pub const CONNECT_LOG_INTERVAL_MS: i64 = 1000;
/// HTTP(S) response stall timeout.
pub const RESPONSE_STALL_TIMEOUT_MS: i64 = 30_000;

/// Returns the next available HTTP source port.
pub fn next_src_port() -> u16 {
	loop {
		let port = NEXT_EPHEMERAL_PORT.fetch_add(1, Ordering::Relaxed);
		if port >= 65535 {
			NEXT_EPHEMERAL_PORT.store(49152, Ordering::Relaxed);
		}
		if port >= 49152 {
			return port;
		}
	}
}

/// Fetch the specified URL
pub fn fetch(
	iface: &mut Interface,
	device: &mut VirtioNet,
	sockets: &mut SocketSet<'_>,
	url: &str,
	now: Instant
) -> Result<HttpResult, NullexError> {
	let mut current = ParsedUrl::parse(url)?;
	let mut redirects = 0u8;
	let mut https = current.scheme == Scheme::Https;

	loop {
		if redirects > 5 {
			return Err(NullexError::TooManyRedirects);
		}

		let dst_ip = resolve(&current.host)?;
		if !https {
			match do_fetch_once(iface, device, sockets, &current, dst_ip, now)? {
				FetchStep::Complete(result) => return Ok(result),
				FetchStep::Redirect(location) => {
					serial_println!("[HTTP] redirect to {}", location);
					current = current.resolve_redirect(&location)?;

					if current.scheme == Scheme::Https {
						serial_println!("[HTTP] Redirect requires HTTPS");
						https = true;
					}

					redirects += 1;
				}
			}
		} else {
			match do_https_fetch_once(iface, device, sockets, &current, dst_ip, now)? {
				FetchStep::Complete(result) => return Ok(result),
				FetchStep::Redirect(location) => {
					serial_println!("[HTTPS] redirect to {}", location);
					current = current.resolve_redirect(&location)?;

					if current.scheme == Scheme::Http {
						serial_println!("[HTTPS] Redirect requires HTTP");
						https = false;
					}

					redirects += 1
				}
			}
		}
	}
}

fn do_fetch_once(
	iface: &mut Interface,
	device: &mut VirtioNet,
	sockets: &mut SocketSet<'_>,
	current: &ParsedUrl,
	dst_ip: [u8; 4],
	now: Instant
) -> Result<FetchStep, NullexError> {
	let src_port = next_src_port();
	let conn = TcpConnection::new(sockets);
	conn.connect(iface, sockets, dst_ip, current.port, src_port)?;
	serial_println!(
		"[HTTP] Connecting to {}:{} (src_port={})",
		current.host,
		current.port,
		src_port
	);

	let mut timestamp = now;
	let connect_started = now;
	let mut last_log_ms = now.total_millis();

	loop {
		TcpConnection::poll(iface, device, sockets, timestamp);

		let state = sockets.get::<Socket>(conn.handle).state();
		match state {
			smoltcp::socket::tcp::State::Established => break,
			smoltcp::socket::tcp::State::Closed | smoltcp::socket::tcp::State::TimeWait => {
				serial_println!("[HTTP] TCP state: {:?}, aborting", state);
				return Err(NullexError::TcpConnectionFailed);
			}
			_ => {}
		}

		let elapsed = elapsed_ms(connect_started, timestamp);
		let now_ms = timestamp.total_millis();
		if now_ms.saturating_sub(last_log_ms) >= CONNECT_LOG_INTERVAL_MS {
			serial_println!("[HTTP] TCP state: {:?} ({}ms)", state, elapsed);
			last_log_ms = now_ms;
		}

		if elapsed >= CONNECT_TIMEOUT_MS {
			serial_println!("[HTTP] Connect timed out");
			conn.close(sockets);
			return Err(NullexError::TcpConnectionFailed);
		}

		timestamp = crate::rtc::rtc_instant();
		spin_loop();
	}

	let request = alloc::format!(
		"GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Nullex/0.1\r\nConnection: close\r\nAccept: */*\r\n\r\n",
		current.path,
		current.host
	);
	conn.send(sockets, request.as_bytes())?;
	serial_println!(
		"[HTTP] Request sent ({} bytes): GET {} HTTP/1.1gg",
		request.len(),
		current.path
	);

	let mut header_buf = Vec::with_capacity(4096);
	let mut recv_buf = [0u8; HTTP_RECV_CHUNK_SIZE];
	let mut last_progress = timestamp;

	loop {
		timestamp = crate::rtc::rtc_instant();
		TcpConnection::poll(iface, device, sockets, timestamp);

		let read = conn.recv_into(sockets, &mut recv_buf)?;
		if read > 0 {
			last_progress = timestamp;
			let chunk = &recv_buf[..read];
			header_buf.extend_from_slice(chunk);

			let Some(sep) = header_buf.windows(4).position(|w| w == b"\r\n\r\n") else {
				continue;
			};

			let header_section = String::from(
				str::from_utf8(&header_buf[..sep]).map_err(|_| NullexError::HttpInvalidResponse)?
			);
			let response_headers = ResponseHeaders::parse(header_section.as_bytes())?;
			let initial_body = &header_buf[sep + 4..];

			if response_headers.is_redirect() {
				let location = response_headers
					.location
					.clone()
					.ok_or(NullexError::HttpInvalidResponse)?;
				conn.close(sockets);
				return Ok(FetchStep::Redirect(location));
			}

			let result = match classify(&response_headers, current) {
				ResponseKind::Download => {
					let filename = resolve_filename(&response_headers, current);
					let bytes_written = if response_headers.transfer_encoding_chunked {
						serial_println!(
							"[HTTP] Chunked download to '{}' needs buffered decode",
							filename
						);
						let body = collect_page_body(
							iface,
							device,
							sockets,
							&conn,
							response_headers.content_length,
							true,
							initial_body,
							&mut timestamp
						)?;
						write_complete_download(&filename, &body)?
					} else {
						serial_println!("[HTTP] Streaming download to '{}'", filename);
						stream_download_body(
							iface,
							device,
							sockets,
							&conn,
							&filename,
							response_headers.content_length,
							initial_body,
							&mut timestamp
						)?
					};
					HttpResult::Download {
						status_code: response_headers.status_code,
						filename,
						bytes_written
					}
				}
				ResponseKind::Page => {
					let body = collect_page_body(
						iface,
						device,
						sockets,
						&conn,
						response_headers.content_length,
						response_headers.transfer_encoding_chunked,
						initial_body,
						&mut timestamp
					)?;
					let body =
						String::from_utf8(body).map_err(|_| NullexError::HttpInvalidResponse)?;
					HttpResult::Page {
						status_code: response_headers.status_code,
						body
					}
				}
			};

			conn.close(sockets);
			return Ok(FetchStep::Complete(result));
		}

		if socket_finished(sockets, &conn) {
			conn.close(sockets);
			return Err(NullexError::HttpInvalidResponse);
		}

		if elapsed_ms(last_progress, timestamp) >= RESPONSE_STALL_TIMEOUT_MS {
			conn.close(sockets);
			return Err(NullexError::Timeout);
		}

		spin_loop();
	}
}

fn stream_download_body(
	iface: &mut Interface,
	device: &mut VirtioNet,
	sockets: &mut SocketSet<'_>,
	conn: &TcpConnection,
	filename: &str,
	content_length: Option<usize>,
	initial_body: &[u8],
	timestamp: &mut Instant
) -> Result<usize, NullexError> {
	let mut writer = match content_length {
		Some(expected) => FileSystemDownloadedFileWriter::create_with_capacity(filename, expected)?,
		None => FileSystemDownloadedFileWriter::create(filename)?
	};
	let mut bytes_written = 0usize;

	if write_download_chunk(
		&mut writer,
		content_length,
		&mut bytes_written,
		initial_body
	)? {
		writer.finish()?;
		return Ok(writer.bytes_written());
	}

	let mut recv_buf = [0u8; HTTP_RECV_CHUNK_SIZE];
	let mut last_progress = *timestamp;

	loop {
		*timestamp = crate::rtc::rtc_instant();
		TcpConnection::poll(iface, device, sockets, *timestamp);

		let read = match conn.recv_into(sockets, &mut recv_buf) {
			Ok(read) => read,
			Err(e) => {
				writer.abort();
				return Err(e);
			}
		};

		if read > 0 {
			last_progress = *timestamp;
			let chunk = &recv_buf[..read];
			if let Err(e) =
				write_download_chunk(&mut writer, content_length, &mut bytes_written, chunk)
			{
				writer.abort();
				return Err(e);
			}

			if content_length
				.map(|expected| bytes_written >= expected)
				.unwrap_or(false)
			{
				break;
			}

			*timestamp = crate::rtc::rtc_instant();
			TcpConnection::poll(iface, device, sockets, *timestamp);
			continue;
		}

		if socket_finished(sockets, conn) {
			break;
		}

		if elapsed_ms(last_progress, *timestamp) >= RESPONSE_STALL_TIMEOUT_MS {
			break;
		}

		spin_loop();
	}

	if let Some(expected) = content_length
		&& bytes_written != expected
	{
		serial_println!(
			"[HTTP] Content-Length mismatch: expected {} got {}",
			expected,
			bytes_written
		);
		writer.abort();
		return Err(NullexError::DownloadIncomplete);
	}

	writer.finish()?;
	Ok(writer.bytes_written())
}

fn write_complete_download(filename: &str, body: &[u8]) -> Result<usize, NullexError> {
	let mut writer = FileSystemDownloadedFileWriter::create_with_capacity(filename, body.len())?;
	if let Err(e) = writer.write(body) {
		writer.abort();
		return Err(e);
	}
	writer.finish()?;
	Ok(writer.bytes_written())
}

fn collect_page_body(
	iface: &mut Interface,
	device: &mut VirtioNet,
	sockets: &mut SocketSet<'_>,
	conn: &TcpConnection,
	content_length: Option<usize>,
	chunked: bool,
	initial_body: &[u8],
	timestamp: &mut Instant
) -> Result<Vec<u8>, NullexError> {
	let mut body = Vec::new();
	let mut done = append_body_chunk(&mut body, content_length, initial_body);
	let mut recv_buf = [0u8; HTTP_RECV_CHUNK_SIZE];
	let mut last_progress = *timestamp;

	while !done {
		*timestamp = crate::rtc::rtc_instant();
		TcpConnection::poll(iface, device, sockets, *timestamp);

		let read = conn.recv_into(sockets, &mut recv_buf)?;
		if read > 0 {
			last_progress = *timestamp;
			done = append_body_chunk(&mut body, content_length, &recv_buf[..read]);

			*timestamp = crate::rtc::rtc_instant();
			TcpConnection::poll(iface, device, sockets, *timestamp);
			continue;
		}

		if socket_finished(sockets, conn) {
			break;
		}

		if elapsed_ms(last_progress, *timestamp) >= RESPONSE_STALL_TIMEOUT_MS {
			break;
		}

		spin_loop();
	}

	if let Some(expected) = content_length
		&& body.len() != expected
	{
		return Err(NullexError::DownloadIncomplete);
	}

	if chunked {
		decode_chunked(&body)
	} else {
		Ok(body)
	}
}

fn write_download_chunk(
	writer: &mut FileSystemDownloadedFileWriter,
	content_length: Option<usize>,
	bytes_written: &mut usize,
	data: &[u8]
) -> Result<bool, NullexError> {
	let write_len = match content_length {
		Some(expected) => expected.saturating_sub(*bytes_written).min(data.len()),
		None => data.len()
	};

	if write_len > 0 {
		writer.write(&data[..write_len])?;
		*bytes_written += write_len;
	}

	Ok(content_length
		.map(|expected| *bytes_written >= expected)
		.unwrap_or(false))
}

fn append_body_chunk(body: &mut Vec<u8>, content_length: Option<usize>, data: &[u8]) -> bool {
	let write_len = match content_length {
		Some(expected) => expected.saturating_sub(body.len()).min(data.len()),
		None => data.len()
	};

	if write_len > 0 {
		body.extend_from_slice(&data[..write_len]);
	}

	content_length
		.map(|expected| body.len() >= expected)
		.unwrap_or(false)
}

fn socket_finished(sockets: &SocketSet<'_>, conn: &TcpConnection) -> bool {
	let socket = sockets.get::<Socket>(conn.handle);
	!socket.is_active()
		|| matches!(
			socket.state(),
			smoltcp::socket::tcp::State::CloseWait
				| smoltcp::socket::tcp::State::TimeWait
				| smoltcp::socket::tcp::State::Closed
		)
}

/// Perform the `GET` HTTP request.
pub fn http_get(
	iface: &mut Interface,
	device: &mut VirtioNet,
	sockets: &mut SocketSet<'_>,
	dst_ip: [u8; 4],
	dst_port: u16,
	host: &str,
	path: &str,
	now: Instant
) -> Result<HttpResponse, NullexError> {
	let (status_code, _headers, body) =
		do_request(iface, device, sockets, dst_ip, dst_port, host, path, now)?;
	Ok(HttpResponse {
		status_code,
		body
	})
}

fn do_request(
	iface: &mut Interface,
	device: &mut VirtioNet,
	sockets: &mut SocketSet<'_>,
	dst_ip: [u8; 4],
	dst_port: u16,
	host: &str,
	path: &str,
	now: Instant
) -> Result<(u16, String, Vec<u8>), NullexError> {
	let src_port = next_src_port();

	let conn = TcpConnection::new(sockets);
	conn.connect(iface, sockets, dst_ip, dst_port, src_port)?;
	serial_println!(
		"[HTTP] Connecting to {}:{} (src_port={})",
		host,
		dst_port,
		src_port
	);

	let mut timestamp = now;
	let connect_started = now;
	let mut last_log_ms = now.total_millis();

	// connection loop
	loop {
		TcpConnection::poll(iface, device, sockets, timestamp);

		let state = sockets.get::<Socket>(conn.handle).state();
		match state {
			smoltcp::socket::tcp::State::Established => break,
			smoltcp::socket::tcp::State::Closed | smoltcp::socket::tcp::State::TimeWait => {
				serial_println!("[HTTP] TCP state: {:?} — aborting", state);
				return Err(NullexError::TcpConnectionFailed);
			}
			_ => {}
		}

		let elapsed = elapsed_ms(connect_started, timestamp);
		let now_ms = timestamp.total_millis();
		if now_ms.saturating_sub(last_log_ms) >= CONNECT_LOG_INTERVAL_MS {
			serial_println!("[HTTP] TCP state: {:?} ({}ms)", state, elapsed);
			last_log_ms = now_ms;
		}

		if elapsed >= CONNECT_TIMEOUT_MS {
			serial_println!("[HTTP] Connect timed out");
			conn.close(sockets);
			return Err(NullexError::TcpConnectionFailed);
		}

		timestamp = crate::rtc::rtc_instant();
		spin_loop();
	}
	serial_println!("[HTTP] Connected on src_port={}", src_port);

	let request = alloc::format!(
		"GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Nullex/0.1\r\nConnection: close\r\nAccept: */*\r\n\r\n",
		path,
		host
	);
	conn.send(sockets, request.as_bytes())?;
	serial_println!(
		"[HTTP] Request sent ({} bytes): GET {} HTTP/1.1",
		request.len(),
		path
	);

	// receive loop
	let mut raw_response: Vec<u8> = Vec::with_capacity(4096);
	let mut recv_buf = [0u8; HTTP_RECV_CHUNK_SIZE];
	let mut last_progress = timestamp;

	let mut headers_parsed = false;
	let mut content_length: Option<usize> = None;
	let mut header_end: usize = 0;
	let mut done = false;

	while !done {
		// inner drain loop
		loop {
			// poll so that smoltcp and see the NIC frames
			timestamp = crate::rtc::rtc_instant();
			TcpConnection::poll(iface, device, sockets, timestamp);

			let read = conn.recv_into(sockets, &mut recv_buf)?;
			if read == 0 {
				break; // buffer fully drained for now
			}

			last_progress = timestamp;
			raw_response.extend_from_slice(&recv_buf[..read]);

			// parse headers the first time we see \r\n\r\n
			if !headers_parsed {
				if let Some(sep) = raw_response.windows(4).position(|w| w == b"\r\n\r\n") {
					if let Ok(hdr) = str::from_utf8(&raw_response[..sep]) {
						content_length = find_content_length(hdr);
						serial_println!(
							"[HTTP] Headers parsed. Content-Length: {:?}",
							content_length
						);
					}
					header_end = sep + 4;
					headers_parsed = true;
				}
			}

			// Early exit as soon as we have all the body bytes.
			// Critical for octet-stream downloads where the server may not
			// send a FIN promptly after the last data segment.
			if let Some(expected) = content_length {
				let body_received = raw_response.len().saturating_sub(header_end);
				if body_received >= expected {
					serial_println!(
						"[HTTP] Content-Length satisfied: {}/{} bytes",
						body_received,
						expected
					);
					done = true;
					break;
				}
			}

			// Poll immediately after draining so smoltcp sends the updated
			// window advertisement (ACK + new Win=) back to the server right
			// away, keeping the data flowing.
			timestamp = crate::rtc::rtc_instant();
			TcpConnection::poll(iface, device, sockets, timestamp);
		}

		if done {
			break;
		}

		// ── Check connection state ───────────────────────────────────────────
		let (finished, half_closed) = {
			let socket = sockets.get::<Socket>(conn.handle);
			let state = socket.state();
			(
				!socket.is_active(),
				matches!(
					state,
					smoltcp::socket::tcp::State::CloseWait
						| smoltcp::socket::tcp::State::TimeWait
						| smoltcp::socket::tcp::State::Closed
				)
			)
		};

		if finished || half_closed {
			serial_println!(
				"[HTTP] Server closed connection ({} bytes total)",
				raw_response.len()
			);
			break;
		}

		// ── Stall detection ──────────────────────────────────────────────────
		if elapsed_ms(last_progress, timestamp) >= RESPONSE_STALL_TIMEOUT_MS {
			let satisfied = content_length
				.map(|expected| raw_response.len().saturating_sub(header_end) >= expected)
				.unwrap_or(false);

			if satisfied {
				serial_println!("[HTTP] Content-Length satisfied on stall timeout, done");
			} else {
				serial_println!(
					"[HTTP] Stall timeout with {} bytes received (Content-Length: {:?})",
					raw_response.len(),
					content_length
				);
			}
			break;
		}

		spin_loop();
	}

	conn.close(sockets);
	serial_println!(
		"[HTTP] Connection closed, {} bytes total",
		raw_response.len()
	);

	// ── Parse response ───────────────────────────────────────────────────────
	let sep = raw_response
		.windows(4)
		.position(|w| w == b"\r\n\r\n")
		.ok_or(NullexError::HttpInvalidResponse)?;

	let header_section = String::from(
		str::from_utf8(&raw_response[..sep]).map_err(|_| NullexError::HttpInvalidResponse)?
	);

	let status_code = header_section
		.lines()
		.next()
		.and_then(|line| line.split_whitespace().nth(1))
		.and_then(|code| code.parse::<u16>().ok())
		.ok_or(NullexError::HttpInvalidResponse)?;

	let raw_body = &raw_response[sep + 4..];

	let body = if header_section
		.to_ascii_lowercase()
		.contains("transfer-encoding: chunked")
	{
		decode_chunked(raw_body)?
	} else {
		raw_body.to_vec()
	};

	serial_println!("[HTTP] Status: {}, body: {} bytes", status_code, body.len());
	Ok((status_code, header_section, body))
}

fn find_header(headers: &str, name: &str) -> Option<String> {
	for line in headers.lines().skip(1) {
		let Some(colon) = line.find(':') else {
			continue
		};
		if line[..colon].trim().eq_ignore_ascii_case(name) {
			return Some(String::from(line[colon + 1..].trim()));
		}
	}
	None
}

fn find_content_length(headers: &str) -> Option<usize> {
	find_header(headers, "content-length").and_then(|v| v.parse::<usize>().ok())
}
