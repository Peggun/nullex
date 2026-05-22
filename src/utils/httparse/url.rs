//!
//! utils/httparse/url.rs
//! 
//! Utilities for handling the URL
//! 

use alloc::string::String;

use crate::error::NullexError;

/// The HTTP Scheme
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scheme {
	/// HTTP
	Http,
	/// HTTPS
	Https
}

impl Scheme {
	/// The default HTTP port for each protocol.
	pub fn default_port(&self) -> u16 {
		match self {
			Scheme::Http => 80,
			Scheme::Https => 443
		}
	}
}

/// Structure representing a parsed URL.
pub struct ParsedUrl {
	/// HTTP or HTTPS
	pub scheme: Scheme,
	/// Hostname of the URL
	pub host: String,
	/// Port of the URL
	pub port: u16,
	/// Path of the URL
	pub path: String
}

impl ParsedUrl {
	/// Parse a URL and return the result.
	pub fn parse(url: &str) -> Result<ParsedUrl, NullexError> {
		// scheme
		let (scheme, after_scheme) = if let Some(rest) = url.strip_prefix("https://") {
			(Scheme::Https, rest)
		} else if let Some(rest) = url.strip_prefix("http://") {
			(Scheme::Http, rest)
		} else {
			return Err(NullexError::InvalidUrl);
		};

		// auth vs path
		let (authority, path) = match after_scheme.find('/') {
			Some(idx) => (&after_scheme[..idx], &after_scheme[idx..]),
			None => (after_scheme, "/")
		};

		// host & port (optional)
		// use rfind as ipv6 literals dont break on first colon
		let (host, port) = match authority.rfind(':') {
			Some(colon) => {
				let port_str = &authority[colon + 1..];
				let port = port_str
					.parse::<u16>()
					.map_err(|_| NullexError::InvalidUrl)?;
				(&authority[..colon], port)
			}
			None => (authority, scheme.default_port())
		};

		if host.is_empty() {
			return Err(NullexError::InvalidUrl);
		}

		Ok(ParsedUrl {
			scheme,
			host: String::from(host),
			port,
			path: String::from(path)
		})
	}

	/// Resolve all redirects from a URL.
	pub fn resolve_redirect(&self, location: &str) -> Result<ParsedUrl, NullexError> {
		if location.starts_with("http://") || location.starts_with("https://") {
			Self::parse(location)
		} else if location.starts_with('/') {
			Ok(ParsedUrl {
				scheme: self.scheme.clone(),
				host: self.host.clone(),
				port: self.port,
				path: String::from(location)
			})
		} else {
			let base = match self.path.rfind('/') {
				Some(idx) => &self.path[..=idx],
				None => "/"
			};

			let mut resolved = String::from(base);
			resolved.push_str(location);
			Ok(ParsedUrl {
				scheme: self.scheme.clone(),
				host: self.host.clone(),
				port: self.port,
				path: resolved
			})
		}
	}
}

#[cfg(feature = "test")]
mod tests {
	use super::*;
	use crate::utils::ktest::TestError;

	fn parses_http() -> Result<(), TestError> {
		let u = ParsedUrl::parse("http://example.com/foo/bar").unwrap();
		assert_eq!(u.scheme, Scheme::Http);
		assert_eq!(u.host, "example.com");
		assert_eq!(u.port, 80);
		assert_eq!(u.path, "/foo/bar");

		Ok(())
	}

	fn parses_https_custom_port() -> Result<(), TestError> {
		let u = ParsedUrl::parse("https://example.com:8443/path?q=1").unwrap();
		assert_eq!(u.scheme, Scheme::Https);
		assert_eq!(u.port, 8443);
		assert_eq!(u.path, "/path?q=1");

		Ok(())
	}

	fn parses_no_path() -> Result<(), TestError> {
		let u = ParsedUrl::parse("http://example.com").unwrap();
		assert_eq!(u.path, "/");

		Ok(())
	}

	fn rejects_missing_scheme() -> Result<(), TestError> {
		assert!(ParsedUrl::parse("example.com/foo").is_err());

		Ok(())
	}

	fn resolve_absolute_redirect() -> Result<(), TestError> {
		let base = ParsedUrl::parse("http://a.com/old").unwrap();
		let r = base.resolve_redirect("https://b.com/new").unwrap();
		assert_eq!(r.host, "b.com");
		assert_eq!(r.scheme, Scheme::Https);

		Ok(())
	}

	fn resolve_path_redirect() -> Result<(), TestError> {
		let base = ParsedUrl::parse("http://a.com/old/page").unwrap();
		let r = base.resolve_redirect("/new/page").unwrap();
		assert_eq!(r.host, "a.com");
		assert_eq!(r.path, "/new/page");

		Ok(())
	}

	crate::create_test!(parses_http);
	crate::create_test!(parses_https_custom_port);
	crate::create_test!(parses_no_path);
	crate::create_test!(rejects_missing_scheme);
	crate::create_test!(resolve_absolute_redirect);
	crate::create_test!(resolve_path_redirect);
}
