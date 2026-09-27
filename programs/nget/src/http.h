#ifndef NGET_HTTP_H
#define NGET_HTTP_H

#include "nget.h"

int http_download_to_buf(const char *url, char *out_buf, size_t max_len,
                         size_t *out_len);
int http_download_to_file(const char *url, const char *file_path);
char *download_index(const char *base_url);

#endif