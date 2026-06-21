//!
//! utils/httparse/response.rs
//!
//! Utilities for handling the response of a HTTP request.

use alloc::string::String;

use crate::{
	serial_println,
	utils::httparse::{headers::ResponseHeaders, url::ParsedUrl}
};

/// The result of a HTTP request.
#[derive(Debug, Clone)]
pub enum HttpResult {
	/// A HTML page
	Page {
		/// Status Code
		status_code: u16,
		/// Body
		body: String
	},

	/// A download
	Download {
		/// Status Code
		status_code: u16,
		/// Filename
		filename: String,
		/// Bytes written
		bytes_written: usize
	}
}

/// Decides whether a response should be treated as a page or a file download.
///
/// Rules (in order):
///   1. `Content-Disposition: attachment`  -> always a download
///   2. Content-Type is a text type        -> page
///   3. Content-Type is anything else      -> download
///   4. No Content-Type at all             -> sniff the URL path extension;
///      unknown/no extension -> download
pub fn classify(headers: &ResponseHeaders, url: &ParsedUrl) -> ResponseKind {
	// rule 1
	if headers.is_attachment {
		return ResponseKind::Download;
	}

	match headers.content_type.as_deref() {
		// rule 2
		Some(ct) if is_text_type(ct) => ResponseKind::Page,

		// rule 3
		Some(_) => ResponseKind::Download,

		// rule 4
		None => {
			if url_looks_like_download(&url.path) {
				ResponseKind::Download
			} else {
				ResponseKind::Page
			}
		}
	}
}

/// The type of HTTP response.
#[derive(Debug, PartialEq)]
pub enum ResponseKind {
	/// A HTML page
	Page,
	/// A file download
	Download
}

/// If the URL's Content-Type is a type of a text
pub fn is_text_type(ct: &str) -> bool {
	serial_println!("{}", ct);
	ct.starts_with("text/")
		|| ct == "application/json"
		|| ct == "application/xml"
		|| ct == "application/xhtml+xml"
		|| ct == "application/octect-stream"
}

/// If the URL looks like a download.
pub fn url_looks_like_download(path: &str) -> bool {
	let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();

	matches!(
		ext.as_str(),
		"bin"
			| "exe" | "zip"
			| "tar" | "gz"
			| "bz2" | "xz"
			| "7z" | "iso"
			| "img" | "dmg"
			| "pdf" | "doc"
			| "docx" | "xls"
			| "xlsx" | "ppt"
			| "pptx" | "mp3"
			| "mp4" | "mkv"
			| "avi" | "mov"
			| "flac" | "wav"
			| "jpg" | "jpeg"
			| "png" | "gif"
			| "webp" | "elf"
	)
}

/// Resolve the filename of a download through the URL and response headers.
pub fn resolve_filename(headers: &ResponseHeaders, url: &ParsedUrl) -> String {
	if let Some(name) = &headers.filename {
		if !name.is_empty() {
			return name.clone();
		}
	}

	let path_only = url.path.split('?').next().unwrap_or(&url.path);
	if let Some(segment) = path_only.rsplit('/').find(|s| !s.is_empty()) {
		if segment.contains('.') || segment.len() > 1 {
			return String::from(segment);
		}
	}

	String::from("download.bin")
}

#[cfg(feature = "test")]
mod tests {
	use super::*;
	use crate::utils::{
		httparse::{headers::ResponseHeaders, url::ParsedUrl},
		ktest::TestError
	};

	fn make_headers(
		content_type: Option<&str>,
		is_attachment: bool,
		filename: Option<&str>
	) -> ResponseHeaders {
		ResponseHeaders {
			status_code: 200,
			content_length: None,
			content_type: content_type.map(String::from),
			filename: filename.map(String::from),
			transfer_encoding_chunked: false,
			location: None,
			is_attachment
		}
	}

	fn url(path: &str) -> ParsedUrl {
		ParsedUrl::parse(&alloc::format!("http://example.com{}", path)).unwrap()
	}

	fn html_is_page() -> Result<(), TestError> {
		let h = make_headers(Some("text/html"), false, None);
		assert_eq!(classify(&h, &url("/")), ResponseKind::Page);

		Ok(())
	}

	fn octet_stream_is_download() -> Result<(), TestError> {
		let h = make_headers(Some("application/octet-stream"), false, None);
		assert_eq!(classify(&h, &url("/file.bin")), ResponseKind::Download);

		Ok(())
	}

	fn attachment_disposition_overrides_text() -> Result<(), TestError> {
		let h = make_headers(Some("text/plain"), true, None);
		assert_eq!(classify(&h, &url("/readme.txt")), ResponseKind::Download);

		Ok(())
	}

	fn no_content_type_bin_extension() -> Result<(), TestError> {
		let h = make_headers(None, false, None);
		assert_eq!(classify(&h, &url("/100MB.bin")), ResponseKind::Download);

		Ok(())
	}

	fn no_content_type_html_path() -> Result<(), TestError> {
		let h = make_headers(None, false, None);
		assert_eq!(classify(&h, &url("/index.html")), ResponseKind::Page);

		Ok(())
	}

	fn filename_from_content_disposition() -> Result<(), TestError> {
		let h = make_headers(Some("application/octet-stream"), true, Some("report.pdf"));
		assert_eq!(resolve_filename(&h, &url("/download?id=5")), "report.pdf");

		Ok(())
	}

	fn filename_from_url_path() -> Result<(), TestError> {
		let h = make_headers(Some("application/octet-stream"), false, None);
		assert_eq!(resolve_filename(&h, &url("/files/100MB.bin")), "100MB.bin");

		Ok(())
	}

	fn filename_from_url_with_query() -> Result<(), TestError> {
		let h = make_headers(None, false, None);
		assert_eq!(
			resolve_filename(&h, &url("/files/setup.exe?v=2")),
			"setup.exe"
		);

		Ok(())
	}

	fn filename_fallback() -> Result<(), TestError> {
		let h = make_headers(None, false, None);
		assert_eq!(resolve_filename(&h, &url("/")), "download.bin");

		Ok(())
	}

	crate::create_test!(html_is_page);
	crate::create_test!(octet_stream_is_download);
	crate::create_test!(attachment_disposition_overrides_text);
	crate::create_test!(no_content_type_bin_extension);
	crate::create_test!(no_content_type_html_path);
	crate::create_test!(filename_from_content_disposition);
	crate::create_test!(filename_from_url_path);
	crate::create_test!(filename_from_url_with_query);
	crate::create_test!(filename_fallback);
}
