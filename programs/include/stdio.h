#ifndef _STDIO_H
#define _STDIO_H

#include "stdarg.h"
#include "stddef.h"

int printf(const char *restrict format, ...);

int vsnprintf(char *s, size_t n, const char *format, va_list args);
int snprintf(char *s, size_t n, const char *format, ...);

char *format_alloc(const char *format, ...);
#define format(...) format_alloc(__VA_ARGS__)

#endif
