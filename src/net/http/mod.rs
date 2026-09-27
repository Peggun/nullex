//!
//! net/http/mod.rs
//!
//! HTTP network request handling.

pub mod httparse;

use alloc::{string::String, vec::Vec};
use core::{
	hint::spin_loop,
	sync::atomic::{AtomicU16, Ordering}
};

use smoltcp::{socket::tcp::Socket, time::Instant};

use crate::{
	error::NullexError,
	net::{
		NET_MANAGER,
		dns::resolve,
		http::httparse::{
			chunked::decode_chunked,
			headers::ResponseHeaders,
			response::{HttpResult, ResponseKind, classify, resolve_filename},
			url::{ParsedUrl, Scheme},
			writer::{DownloadedFileWriter, FileSystemDownloadedFileWriter}
		},
		https::do_https_fetch_once,
		tcp::TcpConnection
	},
	serial_println,
	time::elapsed_ms
};

/// Static reference to the next ephemeral port.
pub static NEXT_EPHEMERAL_PORT: AtomicU16 = AtomicU16::new(50000);

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
pub fn fetch(url: &str, now: Instant) -> Result<HttpResult, NullexError> {
	let mut current = ParsedUrl::parse(url)?;
	let mut redirects = 0u8;
	let mut https = current.scheme == Scheme::Https;

	loop {
		if redirects > 5 {
			return Err(NullexError::TooManyRedirects);
		}

		let dst_ip = resolve(&current.host)?;
		if !https {
			match do_fetch_once(&current, dst_ip, now)? {
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
			match do_https_fetch_once(&current, dst_ip, now)? {
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
	current: &ParsedUrl,
	dst_ip: [u8; 4],
	now: Instant
) -> Result<FetchStep, NullexError> {
	let src_port = next_src_port();
	let conn = TcpConnection::new(Scheme::Http);
	conn.connect(dst_ip, current.port, src_port)?;
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
		TcpConnection::poll(timestamp);

		let mut guard = NET_MANAGER.lock();
		let manager = guard.as_mut().ok_or(NullexError::NetworkNotInitialized)?;
		let state = manager.sockets.get::<Socket>(conn.handle).state();
		core::mem::drop(guard);
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
			conn.close();
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
	conn.send(request.as_bytes())?;
	serial_println!(
		"[HTTP] Request sent ({} bytes): GET {} HTTP/1.1",
		request.len(),
		current.path
	);

	let mut header_buf = Vec::with_capacity(4096);
	let mut recv_buf = [0u8; HTTP_RECV_CHUNK_SIZE];
	let mut last_progress = timestamp;

	loop {
		timestamp = crate::rtc::rtc_instant();
		TcpConnection::poll(timestamp);

		let read = conn.recv_into(&mut recv_buf)?;
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
				conn.close();
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

			conn.close();
			return Ok(FetchStep::Complete(result));
		}

		if socket_finished(&conn) {
			conn.close();
			return Err(NullexError::HttpInvalidResponse);
		}

		if elapsed_ms(last_progress, timestamp) >= RESPONSE_STALL_TIMEOUT_MS {
			conn.close();
			return Err(NullexError::Timeout);
		}

		spin_loop();
	}
}

fn stream_download_body(
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
		TcpConnection::poll(*timestamp);

		let read = match conn.recv_into(&mut recv_buf) {
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
			TcpConnection::poll(*timestamp);
			continue;
		}

		if socket_finished(conn) {
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
		TcpConnection::poll(*timestamp);

		let read = conn.recv_into(&mut recv_buf)?;
		if read > 0 {
			last_progress = *timestamp;
			done = append_body_chunk(&mut body, content_length, &recv_buf[..read]);

			*timestamp = crate::rtc::rtc_instant();
			TcpConnection::poll(*timestamp);
			continue;
		}

		if socket_finished(conn) {
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

pub fn socket_finished(conn: &TcpConnection) -> bool {
	let mut guard = NET_MANAGER.lock();
	let manager = guard
		.as_mut()
		.ok_or(NullexError::NetworkNotInitialized)
		.expect("network not initialized.");

	let socket = manager.sockets.get::<Socket>(conn.handle);
	!socket.is_active()
		|| matches!(
			socket.state(),
			smoltcp::socket::tcp::State::CloseWait
				| smoltcp::socket::tcp::State::TimeWait
				| smoltcp::socket::tcp::State::Closed
		)
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
