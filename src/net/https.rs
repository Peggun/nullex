//!
//! https.rs
//! 
//! HTTPS network request handling.
//! 

use alloc::{string::String, vec::Vec};
use core::hint::spin_loop;

use embedded_io::Write as _;
use embedded_tls::*;
use smoltcp::{
	iface::{Interface, SocketSet},
	socket::tcp::Socket,
	time::Instant
};

use crate::{
	drivers::virtio::net::VirtioNet,
	error::NullexError,
	net::{
		http::{
			CONNECT_LOG_INTERVAL_MS,
			CONNECT_TIMEOUT_MS,
			FetchStep,
			HTTP_RECV_CHUNK_SIZE,
			RESPONSE_STALL_TIMEOUT_MS,
			next_src_port
		},
		tcp::{TcpConnection, TcpIo}
	},
	serial_println,
	utils::{
		httparse::{
			chunked::decode_chunked,
			headers::ResponseHeaders,
			response::{HttpResult, ResponseKind, classify, resolve_filename},
			url::ParsedUrl,
			writer::{DownloadedFileWriter, FileSystemDownloadedFileWriter}
		},
		rng::KernelRng,
		time::elapsed_ms
	}
};

const TLS_RECORD_BUFFER_SIZE: usize = 16_640;

/// 
pub fn do_https_fetch_once(
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
		"[HTTPS] Connecting to {}:{} (src_port={})",
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
				serial_println!("[HTTPS] TCP state: {:?}, aborting", state);
				return Err(NullexError::TcpConnectionFailed);
			}
			_ => {}
		}

		let elapsed = elapsed_ms(connect_started, timestamp);
		let now_ms = timestamp.total_millis();
		if now_ms.saturating_sub(last_log_ms) >= CONNECT_LOG_INTERVAL_MS {
			serial_println!("[HTTPS] TCP state: {:?} ({}ms)", state, elapsed);
			last_log_ms = now_ms;
		}

		if elapsed >= CONNECT_TIMEOUT_MS {
			serial_println!("[HTTPS] Connect timed out");
			conn.close(sockets);
			return Err(NullexError::TcpConnectionFailed);
		}

		timestamp = crate::rtc::rtc_instant();
		spin_loop();
	}

	serial_println!("rng");
	let rng = KernelRng::try_new().expect("entropy init failed");
	serial_println!("rng");

	let config = TlsConfig::new()
		.with_server_name(&current.host)
		.enable_rsa_signatures();

	let provider = UnsecureProvider::new::<Aes128GcmSha256>(rng);

	// Keep TLS record storage off the 64 KiB boot stack. Debug crypto code also
	// has large stack probes, so these buffers cannot live in this frame.
	let mut record_read_buffer = vec![0u8; TLS_RECORD_BUFFER_SIZE];
	let mut record_write_buffer = vec![0u8; TLS_RECORD_BUFFER_SIZE];

	{
		let transport = TcpIo::new(&conn, iface, device, sockets, crate::rtc::rtc_instant);

		let mut tls = blocking::TlsConnection::new(
			transport,
			record_read_buffer.as_mut_slice(),
			record_write_buffer.as_mut_slice()
		);

		tls.open(TlsContext::new(&config, provider))
			.map_err(|_| NullexError::TlsFailed)?;

		let request = alloc::format!(
			"GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: Nullex/0.1\r\nConnection: close\r\nAccept: */*\r\n\r\n",
			current.path,
			current.host
		);
		tls.write_all(request.as_bytes())
			.map_err(|_| NullexError::TlsFailed)?;
		tls.flush().map_err(|_| NullexError::TlsFailed)?;

		serial_println!(
			"[HTTPS] Request sent ({} bytes): GET {} HTTP/1.1",
			request.len(),
			current.path
		);

		let mut header_buf = Vec::with_capacity(4096);
		let mut recv_buf = [0u8; HTTP_RECV_CHUNK_SIZE];

		loop {
			timestamp = crate::rtc::rtc_instant();

			let read = tls
				.read(&mut recv_buf)
				.map_err(|_| NullexError::TlsFailed)?;
			if read > 0 {
				let chunk = &recv_buf[..read];
				header_buf.extend_from_slice(chunk);

				let Some(sep) = header_buf.windows(4).position(|w| w == b"\r\n\r\n") else {
					continue;
				};

				let header_section = String::from(
					str::from_utf8(&header_buf[..sep])
						.map_err(|_| NullexError::HttpInvalidResponse)?
				);
				let response_headers = ResponseHeaders::parse(header_section.as_bytes())?;
				let initial_body = &header_buf[sep + 4..];

				if response_headers.is_redirect() {
					let location = response_headers
						.location
						.clone()
						.ok_or(NullexError::HttpInvalidResponse)?;
					let _ = tls.close();
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
							let body = collect_https_page_body(
								&mut tls,
								response_headers.content_length,
								true,
								initial_body,
								&mut timestamp
							)?;
							write_https_complete_download(&filename, &body)?
						} else {
							serial_println!("[HTTP] Streaming download to '{}'", filename);
							stream_https_download_body(
								&mut tls,
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
						let body = collect_https_page_body(
							&mut tls,
							response_headers.content_length,
							response_headers.transfer_encoding_chunked,
							initial_body,
							&mut timestamp
						)?;
						let body = String::from_utf8(body)
							.map_err(|_| NullexError::HttpInvalidResponse)?;
						HttpResult::Page {
							status_code: response_headers.status_code,
							body
						}
					}
				};

				conn.close(sockets);
				return Ok(FetchStep::Complete(result));
			}
		}
	}
}

fn stream_https_download_body<T>(
	tls: &mut T,
	filename: &str,
	content_length: Option<usize>,
	initial_body: &[u8],
	timestamp: &mut Instant
) -> Result<usize, NullexError>
where
	T: embedded_io::Read<Error = embedded_tls::TlsError>
{
	let mut writer = match content_length {
		Some(expected) => FileSystemDownloadedFileWriter::create_with_capacity(filename, expected)?,
		None => FileSystemDownloadedFileWriter::create(filename)?
	};
	let mut bytes_written = 0usize;

	if write_https_download_chunk(
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

		let read = match tls.read(&mut recv_buf) {
			Ok(read) => read,
			Err(_) => {
				writer.abort();
				return Err(NullexError::TlsFailed);
			}
		};

		if read > 0 {
			last_progress = *timestamp;
			let chunk = &recv_buf[..read];
			if let Err(e) =
				write_https_download_chunk(&mut writer, content_length, &mut bytes_written, chunk)
			{
				writer.abort();
				return Err(e)
			}

			if content_length
				.map(|expected| bytes_written >= expected)
				.unwrap_or(false)
			{
				break;
			}
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

fn write_https_complete_download(filename: &str, body: &[u8]) -> Result<usize, NullexError> {
	let mut writer = FileSystemDownloadedFileWriter::create_with_capacity(filename, body.len())?;
	if let Err(e) = writer.write(body) {
		writer.abort();
		return Err(e);
	}
	writer.finish()?;
	Ok(writer.bytes_written())
}

fn collect_https_page_body<T>(
	tls: &mut T,
	content_length: Option<usize>,
	chunked: bool,
	initial_body: &[u8],
	timestamp: &mut Instant
) -> Result<Vec<u8>, NullexError>
where
	T: embedded_io::Read<Error = embedded_tls::TlsError>
{
	let mut body = Vec::new();
	let mut done = append_https_body_chunk(&mut body, content_length, initial_body);
	let mut recv_buf = [0u8; HTTP_RECV_CHUNK_SIZE];
	let mut last_progress = *timestamp;

	while !done {
		*timestamp = crate::rtc::rtc_instant();

		let read = match tls.read(&mut recv_buf) {
			Ok(read) => read,
			Err(_) => {
				return Err(NullexError::TlsFailed);
			}
		};
		if read > 0 {
			last_progress = *timestamp;
			done = append_https_body_chunk(&mut body, content_length, &recv_buf[..read]);

			*timestamp = crate::rtc::rtc_instant();
			continue;
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

fn write_https_download_chunk(
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

fn append_https_body_chunk(body: &mut Vec<u8>, content_length: Option<usize>, data: &[u8]) -> bool {
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
