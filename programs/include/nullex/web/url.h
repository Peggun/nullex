/**
 * @file url.h
 * @brief HTTP and HTTPS URL parsing and resolution utilities.
 *
 * Provides types and functions for parsing URLs, resolving relative
 * redirects, and releasing dynamically allocated URL components.
 *
 * @note This implementation is a C port of the Rust URL parsing code from
 *       @c src/utils/httparse/url.rs.
 */

#ifndef NULLEX_WEB_URL_H
#define NULLEX_WEB_URL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

/**
 * @brief Supported URL schemes.
 */
typedef enum {
  /** @brief HTTP URL scheme. */
  SCHEME_HTTP,

  /** @brief HTTPS URL scheme. */
  SCHEME_HTTPS
} Scheme;

/**
 * @brief Errors that may occur while parsing or resolving a URL.
 */
typedef enum {
  /** @brief URL operation completed successfully. */
  URL_OK = 0,

  /** @brief The supplied URL is invalid or malformed. */
  URL_INVALID_URL,

  /** @brief A required pointer argument was @c NULL. */
  URL_NULL_POINTER,

  /** @brief Memory allocation required by the operation failed. */
  URL_MALLOC_FAILED,
} UrlError;

/**
 * @brief Represents a parsed HTTP or HTTPS URL.
 *
 * The @c host and @c path members are dynamically allocated and must be
 * released using @c url_free() when the structure is no longer required.
 */
typedef struct {
  /** @brief URL scheme. */
  Scheme scheme;

  /**
   * @brief Null-terminated hostname.
   *
   * Contains the host component of the parsed URL.
   */
  char *host;

  /**
   * @brief TCP port associated with the URL.
   *
   * This is normally the explicitly specified port or the default port
   * associated with @c scheme.
   */
  uint16_t port;

  /**
   * @brief Null-terminated URL path.
   *
   * Contains the path component of the parsed URL.
   */
  char *path;
} ParsedUrl;

/**
 * @brief Get the default TCP port for a URL scheme.
 *
 * @param[in] scheme URL scheme whose default port is required.
 * @return Default TCP port for @p scheme.
 *
 * @retval 80  For @c SCHEME_HTTP.
 * @retval 443 For @c SCHEME_HTTPS.
 */
uint16_t scheme_default_port(Scheme scheme);

/**
 * @brief Parse an HTTP or HTTPS URL.
 *
 * Parses the supplied URL and writes the resulting components into
 * @p out_parsed.
 *
 * @param[in] url Null-terminated URL to parse.
 * @param[out] out_parsed Structure receiving the parsed URL components.
 * @return A @c UrlError value describing the result of the operation.
 *
 * @retval URL_OK The URL was parsed successfully.
 * @retval URL_INVALID_URL The URL is malformed or uses an unsupported format.
 * @retval URL_NULL_POINTER A required argument was @c NULL.
 * @retval URL_MALLOC_FAILED Memory allocation failed while constructing
 *                           the parsed URL.
 *
 * @note Any dynamically allocated members of @p out_parsed should be
 *       released with @c url_free() after use.
 */
UrlError url_parse(const char *url, ParsedUrl *out_parsed);

/**
 * @brief Resolve a URL relative to a base URL.
 *
 * Resolves the supplied redirect location using @p base_url and writes the
 * resulting absolute URL into @p out_parsed.
 *
 * @param[in] base_url Parsed base URL used for resolution.
 * @param[in] location Redirect location to resolve.
 * @param[out] out_parsed Structure receiving the resolved URL.
 * @return A @c UrlError value describing the result of the operation.
 *
 * @retval URL_OK The redirect was resolved successfully.
 * @retval URL_INVALID_URL The location could not be resolved into a valid URL.
 * @retval URL_NULL_POINTER A required argument was @c NULL.
 * @retval URL_MALLOC_FAILED Memory allocation failed while constructing
 *                           the resolved URL.
 *
 * @note Any dynamically allocated members of @p out_parsed should be
 *       released with @c url_free() after use.
 */
UrlError url_resolve_redirect(const ParsedUrl *base_url, const char *location,
                              ParsedUrl *out_parsed);

/**
 * @brief Release memory associated with a parsed URL.
 *
 * Frees dynamically allocated components of the supplied URL and leaves
 * the structure in an unusable or reset state depending on the
 * implementation.
 *
 * @param[in,out] url Parsed URL whose allocated resources should be freed.
 */
void url_free(ParsedUrl *url);

#endif