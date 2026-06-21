// string.h
#ifndef _STRING_H
#define _STRING_H

#include <stddef.h>

size_t strlen(const char *str);
size_t strnlen(const char *s, size_t maxlen);
int strcmp(const char *str1, const char *str2);
char *strcpy(char* __restrict dest, const char* __restrict src);
char *strtok(char *s, const char *delim);
char *strtok_r(char *s, const char *delim, char **save_ptr);
size_t strspn(const char *str, const char *accept);
char *strpbrk(const char *s, const char *accept);
size_t strcspn(const char *s1, const char *s2);
char *strncpy(char *s1, const char *s2, size_t n);

void *memcpy(void *dest, const void *src, size_t len);
void *memset(void *dest, int val, size_t len);

static void trim_newline(char *s);
static void read_line(const char *prompt, char *buffer, size_t len);

#endif