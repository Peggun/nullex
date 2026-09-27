#include <crypto.h>
#include <nullex/fs.h>
#include <nullex/syscalls.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

char *normpath(const char *path, char *out, size_t out_size) {
  if (!path || !out || out_size == 0) {
    return NULL;
  }

  char tmp[MAX_PATH_LEN];
  char *parts[MAX_PARTS];
  size_t count = 0;

  strncpy(tmp, path, sizeof(tmp) - 1);
  tmp[sizeof(tmp) - 1] = '\0';

  char *save = NULL;
  char *tok = strtok_r(tmp, "/", &save);

  while (tok) {
    if (strcmp(tok, ".") == 0) {
      /* skip */
    } else if (strcmp(tok, "..") == 0) {
      if (count > 0) {
        count--;
      }
    } else {
      if (count >= MAX_PARTS) {
        return NULL;
      }
      parts[count++] = tok;
    }
    tok = strtok_r(NULL, "/", &save);
  }

  if (out_size < 2) {
    return NULL;
  }

  size_t pos = 0;
  out[pos++] = '/';

  for (size_t i = 0; i < count; i++) {
    size_t len = strlen(parts[i]);

    if (pos + len + 1 >= out_size) {
      return NULL;
    }

    if (pos > 1) {
      out[pos++] = '/';
    }

    memcpy(out + pos, parts[i], len);
    pos += len;
  }

  if (pos == 1) {
    out[1] = '\0';
  } else {
    out[pos] = '\0';
  }

  return out;
}

char *rslvpath(const char *path, const char *cwd, char *out, size_t out_size) {
  if (!path || !cwd || !out || out_size == 0) {
    return NULL;
  }

  char temp[MAX_PATH_LEN];

  if (path[0] == '/') {
    strncpy(temp, path, sizeof(temp) - 1);
    temp[sizeof(temp) - 1] = '\0';
  } else {
    if (!joinpath(cwd, path, temp, sizeof(temp))) {
      return NULL;
    }
  }

  return normpath(temp, out, out_size);
}

char *joinpath(const char *cwd, const char *path, char *out, size_t out_size) {
  size_t cwd_len = strlen(cwd);
  size_t path_len = strlen(path);

  bool cwd_has_slash = (cwd_len > 0 && cwd[cwd_len - 1] == '/');
  bool path_has_slash = (path_len > 0 && path[0] == '/');
  
  int slash_count = 0;
  if (!cwd_has_slash && !path_has_slash && cwd_len > 0 && path_len > 0) {
      slash_count = 1;
  }
  
  size_t total_len = cwd_len + slash_count + path_len;
  if (total_len + 1 > out_size) return NULL;

  memcpy(out, cwd, cwd_len);
  if (slash_count) out[cwd_len] = '/';
  memcpy(out + cwd_len + slash_count, path, path_len);
  out[total_len] = '\0';

  return out;
}

int compute_sha256_file(const char *filename, char *output_hex) {
  int fd = openf(filename, O_RDONLY);
  if (fd < 0) return -1;

  SHA256_CTX ctx;
  sha256_init(&ctx);

  uint8_t buffer[4096];
  int bytes_read;
  while ((bytes_read = readf(fd, buffer, sizeof(buffer))) > 0) {
    sha256_update(&ctx, buffer, bytes_read);
  }
  closef(fd);

  if (bytes_read < 0) return -1;

  uint8_t hash[32];
  sha256_final(&ctx, hash);

  for (int i = 0; i < 32; i++) {
    output_hex[i * 2] = hex_chars[(hash[i] >> 4) & 0x0F];
    output_hex[i * 2 + 1] = hex_chars[hash[i] & 0x0F];
  }
  output_hex[64] = '\0';

  return 0;
}