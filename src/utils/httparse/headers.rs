//!
//! utils/httparse/headers.rs
//!
//! Utilities for parsing HTTP response headers.

use alloc::string::String;
use core::str::from_utf8;

use crate::error::NullexError;

/// HTTP response headers.
#[derive(Debug)]
pub struct ResponseHeaders {
	/// The HTTP status code returned.
	pub status_code: u16,
	/// The length of the HTTP response.
	pub content_length: Option<usize>,
	/// The type of the HTTP response.
	pub content_type: Option<String>,
	/// The filename of the HTTP response.
	pub filename: Option<String>,
	/// If the HTTP response is an attachment.
	pub is_attachment: bool,
	/// If the HTTP response is chunked.
	pub transfer_encoding_chunked: bool,
	/// The location of the HTTP response.
	pub location: Option<String>
}

impl ResponseHeaders {
	/// Parse URL headers and return them.
	pub fn parse(raw: &[u8]) -> Result<ResponseHeaders, NullexError> {
		let text = from_utf8(raw).map_err(|_| NullexError::HttpInvalidResponse)?;
		let mut lines = text.lines();

		let status_line = lines.next().ok_or(NullexError::HttpInvalidResponse)?;
		let status_code = status_line
			.split_whitespace()
			.nth(1)
			.and_then(|s| s.parse::<u16>().ok())
			.ok_or(NullexError::HttpInvalidResponse)?;

		let mut content_length = None;
		let mut content_type = None;
		let mut filename = None;
		let mut is_attachment = false;
		let mut transfer_encoding_chunked = false;
		let mut location = None;

		for line in lines {
			let Some(colon) = line.find(':') else {
				continue
			};
			let name = line[..colon].trim();
			let value = line[colon + 1..].trim();

			match_header(
				name,
				value,
				&mut content_length,
				&mut content_type,
				&mut filename,
				&mut is_attachment,
				&mut transfer_encoding_chunked,
				&mut location
			);
		}

		Ok(ResponseHeaders {
			status_code,
			content_length,
			content_type,
			filename,
			is_attachment,
			transfer_encoding_chunked,
			location
		})
	}

	/// Whether or not the HTTP response is a redirect.
	pub fn is_redirect(&self) -> bool {
		matches!(self.status_code, 301 | 302 | 303 | 307 | 308)
	}
}

// tee hee fancy format
fn match_header(
	name: &str,
	value: &str,
	content_length: &mut Option<usize>,
	content_type: &mut Option<String>,
	filename: &mut Option<String>,
	is_attachment: &mut bool,
	transfer_encoding_chunked: &mut bool,
	location: &mut Option<String>
) {
	let mut buf = [0u8; 64];
	let lower = to_lowercase_buf(name, &mut buf);

	match lower {
		"content-length" => {
			*content_length = value.parse::<usize>().ok();
		}
		"content-type" => {
			let mime = value.split(';').next().unwrap_or(value).trim();
			*content_type = Some(String::from(mime));
		}
		"transfer-encoding" => {
			if value.eq_ignore_ascii_case("chunked") {
				*transfer_encoding_chunked = true;
			}
		}
		"location" => {
			*location = Some(String::from(value));
		}
		"content-disposition" => {
			let lower = value.to_ascii_lowercase();
			if lower.starts_with("attachment") {
				*is_attachment = true;
			}
			*filename = parse_filename(value);
		}
		_ => {}
	}
}

fn parse_filename(value: &str) -> Option<String> {
	for part in value.split(';') {
		let part = part.trim();
		if let Some(rest) = part
			.strip_prefix("filename=")
			.or_else(|| part.strip_prefix("filename*="))
		{
			let name = rest.trim_matches('"');
			if !name.is_empty() {
				return Some(String::from(name));
			}
		}
	}
	None
}

fn to_lowercase_buf<'a>(s: &str, buf: &'a mut [u8; 64]) -> &'a str {
	let bytes = s.as_bytes();
	let len = bytes.len().min(64);
	for (i, b) in bytes[..len].iter().enumerate() {
		buf[i] = b.to_ascii_lowercase();
	}
	from_utf8(&buf[..len]).unwrap_or("")
}

#[cfg(feature = "test")]
mod tests {
	use super::*;
	use crate::utils::ktest::TestError;

	fn headers(raw: &str) -> ResponseHeaders {
		ResponseHeaders::parse(raw.as_bytes()).unwrap()
	}

	fn parses_status_200() -> Result<(), TestError> {
		let h = headers(
			"HTTP/1.1 200 OK\r\nContent-Length: 42\r\nContent-Type: application/octet-stream"
		);
		assert_eq!(h.status_code, 200);
		assert_eq!(h.content_length, Some(42));
		assert_eq!(h.content_type.as_deref(), Some("application/octet-stream"));

		Ok(())
	}

	fn parses_redirect() -> Result<(), TestError> {
		let h = headers("HTTP/1.1 301 Moved Permanently\r\nLocation: https://example.com/new");
		assert!(h.is_redirect());
		assert_eq!(h.location.as_deref(), Some("https://example.com/new"));

		Ok(())
	}

	fn parses_chunked() -> Result<(), TestError> {
		let h = headers("HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked");
		assert!(h.transfer_encoding_chunked);
		assert_eq!(h.content_length, None);

		Ok(())
	}

	fn parses_content_disposition_quoted() -> Result<(), TestError> {
		let h =
			headers("HTTP/1.1 200 OK\r\nContent-Disposition: attachment; filename=\"report.pdf\"");
		assert_eq!(h.filename.as_deref(), Some("report.pdf"));

		Ok(())
	}

	fn parses_content_disposition_unquoted() -> Result<(), TestError> {
		let h = headers("HTTP/1.1 200 OK\r\nContent-Disposition: attachment; filename=report.pdf");
		assert_eq!(h.filename.as_deref(), Some("report.pdf"));

		Ok(())
	}

	fn strips_content_type_params() -> Result<(), TestError> {
		let h = headers("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8");
		assert_eq!(h.content_type.as_deref(), Some("text/html"));

		Ok(())
	}

	crate::create_test!(parses_status_200);
	crate::create_test!(parses_redirect);
	crate::create_test!(parses_chunked);
	crate::create_test!(parses_content_disposition_quoted);
	crate::create_test!(parses_content_disposition_unquoted);
	crate::create_test!(strips_content_type_params);
}
