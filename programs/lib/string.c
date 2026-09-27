#pragma GCC optimize("O0")

#include <nullex/io.h>
#include <nullex/syscalls.h>
#include <pointer-arith.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

// strtok
static char *olds;

// https://github.com/lattera/glibc/blob/master/string/strlen.c
// this is causing a page fault for some reason.
// size_t strlen(const char *str) {
//     const char *char_ptr;
//     const unsigned long int *longword_ptr;
//     unsigned long int longword, himagic, lomagic;

//     for (char_ptr = str; ((unsigned long int) char_ptr
// 			& (sizeof (longword) - 1)) != 0;
//         ++char_ptr)
//     if (*char_ptr == '\0')
//         return char_ptr - str;

//     longword_ptr = (unsigned long int *)char_ptr;

//     himagic = 0x80808080L;
//     lomagic = 0x01010101L;
//     if (sizeof(longword) > 4) {
//         himagic = ((himagic << 16) << 16) | himagic;
//         lomagic = ((lomagic << 16) << 16) | lomagic;
//     }
//     if (sizeof(longword) > 8) {
//         __builtin_trap();
//     }

//     for (;;) {
//         longword = *longword_ptr++;

//         if (((longword - lomagic) & ~longword & himagic) != 0) {
//             const char *cp = (const char *) (longword_ptr - 1);

//             if (cp[0] == 0)
//                 return cp - str;
//             if (cp[1] == 0)
//                 return cp - str + 1;
//             if (cp[2] == 0)
//                 return cp - str + 2;
//             if (cp[3] == 0)
//                 return cp - str + 3;
//             if (sizeof (longword) > 4) {
//                 if (cp[4] == 0)
//                     return cp - str + 4;
//                 if (cp[5] == 0)
//                     return cp - str + 5;
//                 if (cp[6] == 0)
//                     return cp - str + 6;
//                 if (cp[7] == 0)
//                     return cp - str + 7;
//             }
//         }
//     }
// }

size_t strlen(const char *str) {
  size_t length = 0;
  while (str[length] != '\0') {
    length++;
  }
  return length;
}

// https://github.com/gcc-mirror/gcc/blob/master/libiberty/strnlen.c
size_t strnlen(const char *s, size_t maxlen) {
  size_t i;

  for (i = 0; i < maxlen; ++i)
    if (s[i] == '\0') break;
  return i;
}

// https://github.com/lattera/glibc/blob/master/string/strcmp.c
int strcmp(const char *p1, const char *p2) {
  const unsigned char *s1 = (const unsigned char *)p1;
  const unsigned char *s2 = (const unsigned char *)p2;
  unsigned char c1, c2;

  do {
    c1 = (unsigned char)*s1++;
    c2 = (unsigned char)*s2++;
    if (c1 == '\0') return c1 - c2;
  } while (c1 == c2);

  return c1 - c2;
}

// https://github.com/gcc-mirror/gcc/blob/master/libiberty/strncmp.c
int strncmp(const char *s1, const char *s2, register size_t n) {
  register uint8_t u1, u2;

  while (n-- > 0) {
    u1 = (uint8_t)*s1++;
    u2 = (uint8_t)*s2++;
    if (u1 != u2) return u1 - u2;
    if (u1 == '\0') return 0;
  }

  return 0;
}

// https://github.com/embeddedartistry/libc/blob/master/src/string/strcpy.c
char *strcpy(char *__restrict dest, const char *__restrict src) {
  const size_t length = strlen(src);
  memcpy(dest, src, length + 1);
  return dest;
}

size_t strspn(const char *str, const char *accept) {
    unsigned char table[256] = {0};
    const unsigned char *a = (const unsigned char *)accept;
    
    while (*a) {
        table[*a++] = 1;
    }
    
    const unsigned char *s = (const unsigned char *)str;
    while (*s && table[*s]) {
        s++;
    }
    
    return (const char *)s - str;
}

// https://github.com/lattera/glibc/blob/master/string/strcspn.c
size_t strcspn(const char *s1, const char *s2) {
  const char *p, *spanp;
  char c, sc;

  for (p = s1;;) {
    c = *p++;
    spanp = s2;
    do {
      if ((sc = *spanp++) == c) return (p - 1 - s1);
    } while (sc != 0);
  }
}

// https://github.com/lattera/glibc/blob/master/string/strpbrk.c
char *strpbrk(const char *s, const char *accept) {
  s += strcspn(s, accept);
  return *s ? (char *)s : NULL;
}

// https://github.com/walac/glibc/blob/master/string/strtok.c
char *strtok(char *s, const char *delim) {
  char *token;

  if (s == NULL) s = olds;

  if (s == NULL) return NULL;

  s += strspn(s, delim);
  if (*s == '\0') {
    olds = NULL;
    return NULL;
  }

  token = s;
  s = strpbrk(token, delim);
  if (s == NULL) {
    olds = NULL;
    return token;
  }

  *s = '\0';
  olds = s + 1;
  return token;
}

char *strtok_r(char *s, const char *delim, char **save_ptr) {
  char *token;

  if (s == NULL) s = *save_ptr;

  if (s == NULL) return NULL;

  s += strspn(s, delim);
  if (*s == '\0') {
    *save_ptr = NULL;
    return NULL;
  }

  token = s;
  s = strpbrk(token, delim);
  if (s == NULL) {
    *save_ptr = NULL;
    return token;
  }

  *s = '\0';
  *save_ptr = s + 1;
  return token;
}

// https://github.com/lattera/glibc/blob/master/string/strncpy.c
char *strncpy(char *dest, const char *src, size_t n) {
    size_t i;
    for (i = 0; i < n && src[i] != '\0'; i++) {
        dest[i] = src[i];
    }
    for (; i < n; i++) {
        dest[i] = '\0';
    }
    return dest;
}

// https://github.com/lattera/glibc/blob/master/string/memcpy.c
void *memcpy(void *dest, const void *src, size_t len) {
  unsigned char *d = dest;
  const unsigned char *s = src;
  for (size_t i = 0; i < len; i++) {
    d[i] = s[i];
  }
  return dest;
}

// https://github.com/gcc-mirror/gcc/blob/master/libgcc/memset.c
void *memset(void *dest, int val, size_t len) {
  unsigned char *d = dest;
  unsigned char v = (unsigned char)val;
  for (size_t i = 0; i < len; i++) {
    d[i] = v;
  }
  return dest;
}

// https://github.com/gcc-mirror/gcc/blob/master/libiberty/memchr.c
void *memchr(register const void *src_void, int c, size_t length) {
  const uint8_t *src = (const uint8_t *)src_void;

  while (length-- > 0) {
    if (*src == c) return (void *)src;
    src++;
  }
  return NULL;
}

// https://github.com/lattera/glibc/blob/master/string/strdup.c
// changed to suit the in-band allocator.
char *strdup(const char *s) {
  if (s == NULL) return NULL;

  size_t len = strlen(s);

  char *new = malloc(len + 1);
  if (new == NULL) return NULL;

  memcpy(new, s, len + 1);

  return new;
}

static void trim_newline(char *s) {
  size_t len = strlen(s);
  while (len > 0) {
    char c = s[len - 1];
    if (c != '\n' && c != '\r') {
      break;
    }
    s[len - 1] = '\0';
    len--;
  }
}

char *strchr(const char *s, int c_in) {
    char c = (char)c_in;
    while (*s) {
        if (*s == c) return (char *)s;
        s++;
    }
    return (c == '\0') ? (char *)s : NULL;
}

char *strrchr(const char *s, int c) {
    const char *last = NULL;
    char ch = (char)c;
    while (*s) {
        if (*s == ch) last = s;
        s++;
    }
    if (ch == '\0') return (char *)s;
    return (char *)last;
}

char *strstr(const char *s1, const char *s2) {
  const char *p = s1;
  const size_t len = strlen(s2);

  if (!len) return s1;
  for (; (p = strchr(p, *s2)) != 0; p++) {
    if (strncmp(p, s2, len) == 0) return (char *)p;
  }

  return (0);
}

char *strcat(char *dest, const char *src) {
  strcpy(dest + strlen(dest), src);
  return dest;
}

// https://github.com/gcc-mirror/gcc/blob/master/libiberty/memcmp.c
int memcmp(const void *str1, const void *str2, size_t count) {
  register const unsigned char *s1 = (const unsigned char *)str1;
  register const unsigned char *s2 = (const unsigned char *)str2;

  while (count-- > 0) {
    if (*s1++ != *s2++)
        return s1[-1] - s2[-1] ? -1 : 1;
  }

  return 0;
}

static void read_line(const char *prompt, char *buffer, size_t len) {
  if (len == 0) {
    return;
  }

  buffer[0] = '\0';
  input(prompt, buffer, len);
  trim_newline(buffer);
}

// https://github.com/portfoliocourses/c-example-code/blob/main/endswith.c
bool endswith(char *string, char *end) {
  int string_length = strlen(string);
  int end_length = strlen(end);

  if (end_length > string_length) return false;

  for (int i = 0; i < end_length; i++) {
    if (string[string_length - i] != end[end_length - i]) return false;
  }

  return true;
}

bool startswith(char *string, char *start) {
  int string_length = strlen(string);
  int start_length = strlen(start);

  if (start_length > string_length) return false;

  for (int i = 0; i < start_length; i++)
    if (string[i] != start[i]) return false;

  return true;
}