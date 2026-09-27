#include <nullex/web/url.h>
#include <stdlib.h>
#include <string.h>

// self explanatory, will move eventually, as well as mem_find
static const char *strip_prefix(const char *str, const char *prefix) {
  size_t len = strlen(prefix);

  if (strncmp(str, prefix, len) != 0) return NULL;

  return str + len;
}

// rusts find and rfind.
static const char *mem_find(const char *str, char c) {
  size_t len = strlen(str);
  return (const char *)memchr(str, c, len);
}

static const char *mem_rfind(const char *str, char c) {
  size_t len = strlen(str);
  if (len == 0) return NULL;
  for (size_t i = len; i > 0; i--) {
    if (str[i - 1] == c) {
      return &str[i - 1];
    }
  }
  return NULL;
}

static bool parse_u16_len(const char *str, size_t len, uint16_t *out_val) {
  if (len == 0 || len > 5) return false;
  uint32_t val = 0;
  for (size_t i = 0; i < len; i++) {
    if (str[i] < '0' || str[i] > '9') return false;
    val = val * 10 + (str[i] - '0');
  }
  if (val > 65535) return false;
  *out_val = (uint16_t)val;
  return true;
}

uint16_t scheme_default_port(Scheme scheme) {
  switch (scheme) {
    case SCHEME_HTTP:
      return 80;
    case SCHEME_HTTPS:
      return 443;
  }
  return 0;
}

void url_free(ParsedUrl *url) {
  if (url) {
    free(url->host);
    free(url->path);
    url->host = NULL;
    url->path = NULL;
  }
}

UrlError url_parse(const char *url, ParsedUrl *out_parsed) {
  if (!url || !out_parsed) return URL_NULL_POINTER;

  out_parsed->host = NULL;
  out_parsed->path = NULL;

  Scheme scheme;
  const char *after_scheme = strip_prefix(url, "https://");
  if (after_scheme) {
    scheme = SCHEME_HTTPS;
  } else {
    after_scheme = strip_prefix(url, "http://");
    if (after_scheme) {
      scheme = SCHEME_HTTP;
    } else {
      return URL_INVALID_URL;
    }
  }

  // auth vs path
  const char *authority = after_scheme;
  size_t authority_len = 0;
  const char *path_start = "/";

  const char *slash_idx = strchr(after_scheme, '/');
  if (slash_idx) {
    authority_len = slash_idx - after_scheme;
    path_start = slash_idx;
  } else {
    authority_len = strlen(after_scheme);
  }

  const char *host_start = authority;
  size_t host_len = authority_len;
  uint16_t port = scheme_default_port(scheme);

  const char *colon_idx = NULL;
  for (size_t i = authority_len; i > 0; i--) {
    if (authority[i - 1] == ':') {
      colon_idx = &authority[i - 1];
      break;
    }
  }

  if (colon_idx) {
      const char *port_str = colon_idx + 1;
      size_t port_len = (authority + authority_len) - port_str;
  
      if (!parse_u16_len(port_str, port_len, &port)) {
        return URL_INVALID_URL;
      }
      host_len = colon_idx - authority;
    }

  if (host_len == 0) {
    return URL_INVALID_URL;
  }

  out_parsed->scheme = scheme;
  out_parsed->port = port;

  out_parsed->host = (char *)malloc(host_len + 1);
  if (!out_parsed->host) return URL_MALLOC_FAILED;
  memcpy(out_parsed->host, host_start, host_len);
  out_parsed->host[host_len] = '\0';

  out_parsed->path = strdup(path_start);
  if (!out_parsed->path) {
    free(out_parsed->host);
    return URL_MALLOC_FAILED;
  }

  return URL_OK;
}

UrlError url_resolve_redirect(const ParsedUrl *base, const char *location,
                              ParsedUrl *out_parsed) {
  if (!base || !location || !out_parsed) return URL_NULL_POINTER;

  out_parsed->host = NULL;
  out_parsed->path = NULL;

  if (strip_prefix(location, "http://") || strip_prefix(location, "https://")) {
    return url_parse(location, out_parsed);
  }

  out_parsed->scheme = base->scheme;
  out_parsed->port = base->port;
  out_parsed->host = strdup(base->host);
  if (!out_parsed->host) return URL_MALLOC_FAILED;

  if (location[0] == '/') {
    out_parsed->path = strdup(location);
    if (!out_parsed->path) {
      free(out_parsed->host);
      return URL_MALLOC_FAILED;
    }
    return URL_OK;
  }

  // Relative path redirect logic
  const char *last_slash = strrchr(base->path, '/');
  size_t base_dir_len = 0;

  if (last_slash) {
    base_dir_len = (last_slash - base->path) + 1;
  } else {
    base_dir_len = 1;
  }

  size_t loc_len = strlen(location);
  size_t total_path_len = base_dir_len + loc_len;

  out_parsed->path = (char *)malloc(total_path_len + 1);
  if (!out_parsed->path) {
    free(out_parsed->host);
    return URL_MALLOC_FAILED;
  }

  if (last_slash) {
    memcpy(out_parsed->path, base->path, base_dir_len);
  } else {
    out_parsed->path[0] = '/';
  }

  memcpy(out_parsed->path + base_dir_len, location, loc_len);
  out_parsed->path[total_path_len] = '\0';

  return URL_OK;
}