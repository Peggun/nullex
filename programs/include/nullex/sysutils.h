#ifndef _SYSUTILS_H
#define _SYSUTILS_H

static inline void say_append_char(char **p, char c, char *end) {
    if (*p < end) {
        **p = c;
        (*p)++;
    }
}

static inline void say_append_str(char **p, const char *s, char *end) {
    while (*s && *p < end) {
        **p = *s;
        (*p)++;
        s++;
    }
}

static inline void say_append_strn(char **p, const char *s, int n, char *end) {
    for (int i = 0; i < n && s[i] && *p < end; i++) {
        **p = s[i];
        (*p)++;
    }
}

static inline void say_append_uint(char **p, unsigned long long num, unsigned base, char *end) {
    char nbuf[32];
    int i = 0;

    if (num == 0) {
        nbuf[i++] = '0';
    } else {
        while (num > 0) {
            unsigned digit = (unsigned)(num % base);
            nbuf[i++] = (digit < 10) ? ('0' + digit) : ('a' + digit - 10);
            num /= base;
        }
    }

    for (int j = i - 1; j >= 0; j--) {
        say_append_char(p, nbuf[j], end);
    }
}

static inline void say_append_int(char **p, long long num, unsigned base, char *end) {
    if (num < 0) {
        say_append_char(p, '-', end);
        num = -num;
    }
    say_append_uint(p, (unsigned long long)num, base, end);
}

#endif