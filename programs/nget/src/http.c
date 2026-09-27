#include "http.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "nget.h"
#include "nullex/fs.h"

static char index_buffer[MAX_INDEX_SIZE];

int http_download_to_buf(const char *url, char *out_buf, size_t max_len,
                         size_t *out_len) {
  int sock = csocket();
  if (sock < 0) return -1;
  if (connsock(sock, url, 0) < 0) return -1;

  ParsedUrl parsed_url;
  if (url_parse(url, &parsed_url) != URL_OK) return -1;

  char request[2048];
  snprintf(request, sizeof(request),
           "GET %s HTTP/1.1\r\nHost: %s\r\nUser-Agent: Nget/1.0\r\nConnection: "
           "close\r\n\r\n",
           parsed_url.path, parsed_url.host);

  send(sock, (uint8_t *)request, strlen(request));

  char buf[4096];
  char hdr_buf[8192] = {0};
  int hdr_len = 0;
  int header_complete = 0;
  size_t total_written = 0;

  while (!header_complete) {
    struct nlx_iovec iov;
    iov.iov_base = hdr_buf + hdr_len;
    iov.iov_len = sizeof(hdr_buf) - hdr_len;

    struct nlx_msghdr msg;
    msg.type = NLX_IO_VEC;
    msg.payload.vec.msg_iov = &iov;
    msg.payload.vec.msg_iovcnt = 1;

    int n = recv(sock, &msg);
    if (n <= 0) break;

    hdr_len += n;
    for (int i = 0; i < hdr_len - 3; i++) {
      if (hdr_buf[i] == '\r' && hdr_buf[i + 1] == '\n' &&
          hdr_buf[i + 2] == '\r' && hdr_buf[i + 3] == '\n') {
        header_complete = 1;
        size_t body_start = i + 4;
        size_t body_chunk_len = hdr_len - body_start;

        if (strstr(hdr_buf, "200 OK") == NULL) {
          say("HTTP Request Failed.\n");
          return -1;
        }

        if (body_chunk_len > 0 && total_written + body_chunk_len < max_len) {
          memcpy(out_buf + total_written, hdr_buf + body_start, body_chunk_len);
          total_written += body_chunk_len;
        }
        break;
      }
    }
    if (hdr_len >= (int)sizeof(hdr_buf)) break;
  }

  while (1) {
    struct nlx_iovec iov;
    iov.iov_base = buf;
    iov.iov_len = sizeof(buf);

    struct nlx_msghdr msg;
    msg.type = NLX_IO_VEC;
    msg.payload.vec.msg_iov = &iov;
    msg.payload.vec.msg_iovcnt = 1;

    int n = recv(sock, &msg);
    if (n <= 0) break;
    if (total_written + n < max_len) {
      memcpy(out_buf + total_written, buf, n);
      total_written += n;
    } else {
      break;
    }
  }

  *out_len = total_written;
  closesock(sock);
  return 0;
}

int http_download_to_file(const char *url, const char *filepath) {
  int sock = csocket();
  if (sock < 0) return -1;
  if (connsock(sock, url, 0) < 0) return -1;

  ParsedUrl parsed_url;
  if (url_parse(url, &parsed_url) != URL_OK) return -1;

  char request[2048];
  snprintf(request, sizeof(request),
           "GET %s HTTP/1.1\r\nHost: %s\r\nUser-Agent: Nget/1.0\r\nConnection: "
           "close\r\n\r\n",
           parsed_url.path, parsed_url.host);

  send(sock, (uint8_t *)request, strlen(request));

  int fd = openf(filepath, O_CREAT | O_WRONLY);
  if (fd < 0) {
    say("Failed to open file for writing: %s\n", filepath);
    return -1;
  }

  char buf[4096];
  char hdr_buf[8192] = {0};
  int hdr_len = 0;
  int header_complete = 0;

  while (!header_complete) {
    struct nlx_iovec iov;
    iov.iov_base = hdr_buf + hdr_len;
    iov.iov_len = sizeof(hdr_buf) - hdr_len;

    struct nlx_msghdr msg;
    msg.type = NLX_IO_VEC;
    msg.payload.vec.msg_iov = &iov;
    msg.payload.vec.msg_iovcnt = 1;

    int n = recv(sock, &msg);
    if (n <= 0) break;

    hdr_len += n;
    for (int i = 0; i < hdr_len - 3; i++) {
      if (hdr_buf[i] == '\r' && hdr_buf[i + 1] == '\n' &&
          hdr_buf[i + 2] == '\r' && hdr_buf[i + 3] == '\n') {
        header_complete = 1;
        size_t body_start = i + 4;
        size_t body_chunk_len = hdr_len - body_start;

        if (strstr(hdr_buf, "200 OK") == NULL) {
          say("HTTP Request Failed.\n");
          closef(fd);
          return -1;
        }

        if (body_chunk_len > 0) {
          writef_buf(fd, (uint8_t *)(hdr_buf + body_start), body_chunk_len);
        }
        break;
      }
    }
    if (hdr_len >= (int)sizeof(hdr_buf)) break;
  }

  while (1) {
    struct nlx_iovec iov;
    iov.iov_base = buf;
    iov.iov_len = sizeof(buf);

    struct nlx_msghdr msg;
    msg.type = NLX_IO_VEC;
    msg.payload.vec.msg_iov = &iov;
    msg.payload.vec.msg_iovcnt = 1;

    int n = recv(sock, &msg);
    if (n <= 0) break;
    writef_buf(fd, (uint8_t *)buf, n);
  }

  closesock(sock);
  closef(fd);
  return 0;
}

char *download_index(const char *base_url) {
  char url[512];
  snprintf(url, sizeof(url), "%s/index.json", base_url);

  size_t len = 0;
  if (http_download_to_buf(url, index_buffer, MAX_INDEX_SIZE, &len) == 0) {
    index_buffer[len] = '\0';
    return index_buffer;
  }
  return NULL;
}