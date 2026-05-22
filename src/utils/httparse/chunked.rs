//!
//! utils/httparse/chunked.rs
//! 
//! Utilities for decoding chunked HTTP replies.
//! 

use alloc::vec::Vec;
use core::str::from_utf8;

use crate::error::NullexError;

/// Decode an HTTP/1.1 `Transfer-Encoding: chunked` body.
///
/// Format per RFC 7230 §4.1:
///
///   <hex-size>\r\n
///   <data bytes>\r\n
///   ...
///   0\r\n
///   \r\n          <- optional trailers omitted here
///
/// Returns the decoded body bytes, or `Err` if the encoding is malformed.
pub fn decode_chunked(input: &[u8]) -> Result<Vec<u8>, NullexError> {
	let mut output = Vec::new();
	let mut pos = 0;

	loop {
		let line_end = find_crlf(input, pos).ok_or(NullexError::HttpInvalidResponse)?;
		let size_line =
			from_utf8(&input[pos..line_end]).map_err(|_| NullexError::HttpInvalidResponse)?;

		let hex = size_line.split(';').next().unwrap_or("").trim();
		let chunk_size =
			usize::from_str_radix(hex, 16).map_err(|_| NullexError::HttpInvalidResponse)?;

		pos = line_end + 2; // skips the \r\n

		if chunk_size == 0 {
			break; // terminating chunk
		}

		let end = pos + chunk_size;
		if end > input.len() {
			return Err(NullexError::HttpInvalidResponse);
		}
		output.extend_from_slice(&input[pos..end]);
		pos = end;

		if input.get(pos..pos + 2) != Some(b"\r\n") {
			return Err(NullexError::HttpInvalidResponse);
		}
		pos += 2;
	}

	Ok(output)
}

fn find_crlf(buf: &[u8], from: usize) -> Option<usize> {
	buf[from..]
		.windows(2)
		.position(|w| w == b"\r\n")
		.map(|p| p + from)
}

#[cfg(feature = "test")]
mod tests {
	use super::*;
	use crate::utils::ktest::TestError;

	fn decodes_simple() -> Result<(), TestError> {
		let chunked = b"7\r\nHello, \r\n6\r\nWorld!\r\n0\r\n\r\n";
		let decoded = decode_chunked(chunked).unwrap();
		assert_eq!(decoded, b"Hello, World!");

		Ok(())
	}

	fn decodes_single_chunk() -> Result<(), TestError> {
		let chunked = b"5\r\nhello\r\n0\r\n\r\n";
		assert_eq!(decode_chunked(chunked).unwrap(), b"hello");

		Ok(())
	}

	fn decodes_empty_body() -> Result<(), TestError> {
		let chunked = b"0\r\n\r\n";
		assert_eq!(decode_chunked(chunked).unwrap(), b"");

		Ok(())
	}

	fn rejects_truncated() -> Result<(), TestError> {
		let chunked = b"a\r\nhi\r\n0\r\n\r\n";
		assert!(decode_chunked(chunked).is_err());

		Ok(())
	}

	fn handles_chunk_extensions() -> Result<(), TestError> {
		let chunked = b"5;ext=ignored\r\nhello\r\n0\r\n\r\n";
		assert_eq!(decode_chunked(chunked).unwrap(), b"hello");

		Ok(())
	}

	crate::create_test!(decodes_simple);
	crate::create_test!(decodes_single_chunk);
	crate::create_test!(decodes_empty_body);
	crate::create_test!(rejects_truncated);
	crate::create_test!(handles_chunk_extensions);
}
