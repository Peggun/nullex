#include "ktest.h"
#include "assert.h"

#include <nullex/web/url.h>
#include <string.h>

int test_scheme_default_port(void) {
  T_ASSERT_EQ(scheme_default_port(SCHEME_HTTP), 80);
  T_ASSERT_EQ(scheme_default_port(SCHEME_HTTPS), 443);
  return 0;
}
CREATE_TEST(test_scheme_default_port)

int test_url_parse_http_basic(void) {
  ParsedUrl parsed;
  UrlError err = url_parse("http://example.com/path", &parsed);

  T_ASSERT_EQ(err, URL_OK);
  T_ASSERT_EQ(parsed.scheme, SCHEME_HTTP);
  T_ASSERT_STREQ(parsed.host, "example.com");
  T_ASSERT_EQ(parsed.port, 80);
  T_ASSERT_STREQ(parsed.path, "/path");

  url_free(&parsed);
  return 0;
}
CREATE_TEST(test_url_parse_http_basic)

int test_url_parse_https_is_not_confused_with_http(void) {
  ParsedUrl parsed;
  UrlError err = url_parse("https://example.com/secure", &parsed);

  T_ASSERT_EQ(err, URL_OK);
  T_ASSERT_EQ(parsed.scheme, SCHEME_HTTPS);
  T_ASSERT_STREQ(parsed.host, "example.com");
  T_ASSERT_EQ(parsed.port, 443);
  T_ASSERT_STREQ(parsed.path, "/secure");

  url_free(&parsed);
  return 0;
}
CREATE_TEST(test_url_parse_https_is_not_confused_with_http)

int test_url_parse_explicit_port(void) {
  ParsedUrl parsed;
  UrlError err = url_parse("http://example.com:8080/path", &parsed);

  T_ASSERT_EQ(err, URL_OK);
  T_ASSERT_STREQ(parsed.host, "example.com");
  T_ASSERT_EQ(parsed.port, 8080);

  url_free(&parsed);
  return 0;
}
CREATE_TEST(test_url_parse_explicit_port)

int test_url_parse_root_path(void) {
  ParsedUrl parsed;
  UrlError err = url_parse("http://example.com/", &parsed);

  T_ASSERT_EQ(err, URL_OK);
  T_ASSERT_STREQ(parsed.path, "/");

  url_free(&parsed);
  return 0;
}
CREATE_TEST(test_url_parse_root_path)

int test_url_parse_missing_scheme_is_invalid(void) {
  ParsedUrl parsed;
  UrlError err = url_parse("example.com/path", &parsed);
  T_ASSERT_EQ(err, URL_INVALID_URL);
  return 0;
}
CREATE_TEST(test_url_parse_missing_scheme_is_invalid)

int test_url_parse_unsupported_scheme_is_invalid(void) {
  ParsedUrl parsed;
  UrlError err = url_parse("ftp://example.com/path", &parsed);
  T_ASSERT_EQ(err, URL_INVALID_URL);
  return 0;
}
CREATE_TEST(test_url_parse_unsupported_scheme_is_invalid)

int test_url_parse_null_url_is_null_pointer_error(void) {
  ParsedUrl parsed;
  UrlError err = url_parse(NULL, &parsed);
  T_ASSERT_EQ(err, URL_NULL_POINTER);
  return 0;
}
CREATE_TEST(test_url_parse_null_url_is_null_pointer_error)

int test_url_parse_null_out_param_is_null_pointer_error(void) {
  UrlError err = url_parse("http://example.com", NULL);
  T_ASSERT_EQ(err, URL_NULL_POINTER);
  return 0;
}
CREATE_TEST(test_url_parse_null_out_param_is_null_pointer_error)

int test_url_parse_no_path_defaults_to_root(void) {
  ParsedUrl parsed;
  UrlError err = url_parse("http://example.com", &parsed);

  T_ASSERT_EQ(err, URL_OK);
  T_ASSERT_STREQ(parsed.path, "/");

  url_free(&parsed);
  return 0;
}
CREATE_TEST(test_url_parse_no_path_defaults_to_root)

int test_url_resolve_redirect_absolute_location_overrides_base(void) {
  ParsedUrl base;
  T_ASSERT_EQ(url_parse("http://example.com/old", &base), URL_OK);

  ParsedUrl resolved;
  UrlError err =
      url_resolve_redirect(&base, "https://elsewhere.com/new", &resolved);

  T_ASSERT_EQ(err, URL_OK);
  T_ASSERT_EQ(resolved.scheme, SCHEME_HTTPS);
  T_ASSERT_STREQ(resolved.host, "elsewhere.com");
  T_ASSERT_STREQ(resolved.path, "/new");

  url_free(&base);
  url_free(&resolved);
  return 0;
}
CREATE_TEST(test_url_resolve_redirect_absolute_location_overrides_base)

int test_url_resolve_redirect_absolute_path_keeps_host(void) {
  ParsedUrl base;
  T_ASSERT_EQ(url_parse("http://example.com/old/path", &base), URL_OK);

  ParsedUrl resolved;
  UrlError err = url_resolve_redirect(&base, "/new/path", &resolved);

  T_ASSERT_EQ(err, URL_OK);
  T_ASSERT_EQ(resolved.scheme, SCHEME_HTTP);
  T_ASSERT_STREQ(resolved.host, "example.com");
  T_ASSERT_STREQ(resolved.path, "/new/path");

  url_free(&base);
  url_free(&resolved);
  return 0;
}
CREATE_TEST(test_url_resolve_redirect_absolute_path_keeps_host)

int test_url_resolve_redirect_null_arguments(void) {
  ParsedUrl base;
  T_ASSERT_EQ(url_parse("http://example.com/", &base), URL_OK);

  ParsedUrl resolved;
  T_ASSERT_EQ(url_resolve_redirect(NULL, "/x", &resolved), URL_NULL_POINTER);
  T_ASSERT_EQ(url_resolve_redirect(&base, NULL, &resolved), URL_NULL_POINTER);

  url_free(&base);
  return 0;
}
CREATE_TEST(test_url_resolve_redirect_null_arguments)

int test_url_free_after_parse_does_not_crash(void) {
  ParsedUrl parsed;
  T_ASSERT_EQ(url_parse("http://example.com/cleanup-check", &parsed), URL_OK);
  url_free(&parsed);
  return 0;
}
CREATE_TEST(test_url_free_after_parse_does_not_crash)