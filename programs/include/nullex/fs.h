#ifndef _FS_H
#define _FS_H

#define MAX_PATH_LEN 1024
#define MAX_PARTS 128

#include "../include/stddef.h"

char *normpath(const char *path, char *out, size_t out_size);
char *rslvpath(const char *path, const char *cwd, char *out, size_t out_size);
char *joinpath(const char *cwd, const char *path, char *out, size_t out_size);

#endif