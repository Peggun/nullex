#include "../include/string.h"
#include "../include/stddef.h"
#include "../include/stdint.h"
#include "../include/pointer-arith.h"
#include "../include/nullex/io.h"
#include "../include/nullex/syscalls.h"

// strtok
static char *olds;

// https://github.com/lattera/glibc/blob/master/string/strlen.c
size_t strlen(const char *str) {
    const char *char_ptr;
    const unsigned long int *longword_ptr;
    unsigned long int longword, himagic, lomagic;

    for (char_ptr = str; ((unsigned long int) char_ptr
			& (sizeof (longword) - 1)) != 0;
        ++char_ptr)
    if (*char_ptr == '\0')
        return char_ptr - str;

    longword_ptr = (unsigned long int *)char_ptr;

    himagic = 0x80808080L;
    lomagic = 0x01010101L;
    if (sizeof(longword) > 4) {
        himagic = ((himagic << 16) << 16) | himagic;
        lomagic = ((lomagic << 16) << 16) | lomagic;
    }
    if (sizeof(longword) > 8) {
        __builtin_trap();
    }

    for (;;) {
        longword = *longword_ptr++;

        if (((longword - lomagic) & ~longword & himagic) != 0) {
            const char *cp = (const char *) (longword_ptr - 1);

            if (cp[0] == 0)
                return cp - str;
            if (cp[1] == 0)
                return cp - str + 1;
            if (cp[2] == 0)
                return cp - str + 2;
            if (cp[3] == 0)
                return cp - str + 3;
            if (sizeof (longword) > 4) {
                if (cp[4] == 0)
                    return cp - str + 4;
                if (cp[5] == 0)
                    return cp - str + 5;
                if (cp[6] == 0)
                    return cp - str + 6;
                if (cp[7] == 0)
                    return cp - str + 7;
            }
        }
    }
}

// https://github.com/gcc-mirror/gcc/blob/master/libiberty/strnlen.c
size_t strnlen(const char *s, size_t maxlen) {
    size_t i;

    for (i = 0; i < maxlen; ++i)
        if (s[i] == '\0')
            break;
    return i;
}

// https://github.com/lattera/glibc/blob/master/string/strcmp.c
int strcmp(const char *p1, const char *p2) {
    const unsigned char *s1 = (const unsigned char *)p1;
    const unsigned char *s2 = (const unsigned char *)p2;
    unsigned char c1, c2;

    do
    {
        c1 = (unsigned char)*s1++;
        c2 = (unsigned char)*s2++;
        if (c1 == '\0')
            return c1 - c2;
    } while (c1 == c2);

    return c1 - c2;
}

// https://github.com/embeddedartistry/libc/blob/master/src/string/strcpy.c
char *strcpy(char* __restrict dest, const char* __restrict src) {
    const size_t length = strlen(src);
    memcpy(dest, src, length + 1);
    return dest;
}

// https://github.com/lattera/glibc/blob/master/string/strspn.c
size_t strspn(const char *str, const char *accept) {
    if (accept[0] == '\0')
        return 0;
    if (accept[1] == '\0') {
        const char *a = str;
        for (; *str == *accept; str++)
            ;
        return str - a;
    }

    unsigned char table[256];
    unsigned char *p = memset(table, 0, 64);
    memset(p + 64, 0, 64);
    memset(p + 128, 0, 64);
    memset(p + 192, 0, 64);
    
    unsigned char *s = (unsigned char*) accept;
    do
    {
        p[*s++] = 1;
    } while (*s);

    s = (unsigned char*) str;
    if (!p[s[0]]) return 0;
    if (!p[s[1]]) return 1;
    if (!p[s[2]]) return 2;
    if (!p[s[3]]) return 3;

    s = (unsigned char *)PTR_ALIGN_DOWN(s, 4);

    unsigned int c0, c1, c2, c3;
    do {
        s += 4;
        c0 = p[s[0]];
        c1 = p[s[1]];
        c2 = p[s[2]];
        c3 = p[s[3]];
    } while ((c0 & c1 & c2 & c3) != 0);

    size_t count = s - (unsigned char *) str;
    return (c0 & c1) == 0 ? count + c0 : count + c2 + 2;
}

// https://github.com/lattera/glibc/blob/master/string/strcspn.c
size_t strcspn(const char *s1, const char *s2) {
    const char *p, *spanp;
    char c, sc;

    for (p = s1;;) {
        c = *p++;
        spanp = s2;
        do
        {
            if ((sc = *spanp++) == c)
                return (p - 1 - s1);
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

    if (s == NULL)
        s = olds;

    if (s == NULL)
        return NULL;
    
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

    if (s == NULL)
        s = *save_ptr;

    if (s == NULL)
        return NULL;
    
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
char *strncpy(char *s1, const char *s2, size_t n) {
    size_t size = strnlen(s2, n);
    if (size != n) {
        memset(s1 + size, '\0', n - size);
    }
    return memcpy(s1, s2, size);
}

// to be from
// https://github.com/lattera/glibc/blob/master/string/memcpy.c
void *memcpy(void *dest, const void *src, size_t len) {
    char *d = dest;
    const char *s = src;
    while (len--) {
        *d++ = *s++;
    }
    return dest;
}

// https://github.com/gcc-mirror/gcc/blob/master/libgcc/memset.c
void *memset(void *dest, int val, size_t len) {
    unsigned char *ptr = dest;
    while (len-- > 0)
        *ptr++ = val;
    return dest;
}

static void trim_newline(char *s)
{
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

static void read_line(const char *prompt, char *buffer, size_t len) {
    if (len == 0) {
        return;
    }

    buffer[0] = '\0';
    input(prompt, buffer, len);
    trim_newline(buffer);
}
