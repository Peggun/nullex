#include "../include/stdio.h"
#include "../include/stdarg.h"
#include "../include/stdbool.h"
#include "../include/stddef.h"
#include "../include/stdint.h"
#include "../include/stdlib.h"

#include "../include/nullex/syscalls.h"

typedef struct {
    char *buf;
    size_t cap;
    size_t len;
} FormatBuffer;

static void format_putc(FormatBuffer *out, char c)
{
    if (out->buf != NULL && out->cap > 0 && out->len + 1 < out->cap) {
        out->buf[out->len] = c;
    }
    out->len++;
}

static void format_puts_n(FormatBuffer *out, const char *s, int max_len)
{
    if (s == NULL) {
        s = "(null)";
    }

    for (int i = 0; s[i] != '\0'; ++i) {
        if (max_len >= 0 && i >= max_len) {
            break;
        }
        format_putc(out, s[i]);
    }
}

static void format_put_uint(FormatBuffer *out, unsigned long long num, unsigned base, bool uppercase)
{
    char tmp[32];
    int len = 0;

    if (num == 0) {
        tmp[len++] = '0';
    } else {
        while (num > 0) {
            unsigned digit = (unsigned)(num % base);
            if (digit < 10) {
                tmp[len++] = (char)('0' + digit);
            } else {
                tmp[len++] = (char)((uppercase ? 'A' : 'a') + digit - 10);
            }
            num /= base;
        }
    }

    for (int i = len - 1; i >= 0; --i) {
        format_putc(out, tmp[i]);
    }
}

static void format_put_int(FormatBuffer *out, long long num)
{
    if (num < 0) {
        format_putc(out, '-');
        unsigned long long magnitude = (unsigned long long)(-(num + 1)) + 1;
        format_put_uint(out, magnitude, 10, false);
    } else {
        format_put_uint(out, (unsigned long long)num, 10, false);
    }
}

int vsnprintf(char *s, size_t n, const char *format, va_list args)
{
    FormatBuffer out = {
        .buf = s,
        .cap = n,
        .len = 0
    };

    for (const char *f = format; *f != '\0'; ++f) {
        if (*f != '%') {
            format_putc(&out, *f);
            continue;
        }

        ++f;
        if (*f == '\0') {
            format_putc(&out, '%');
            break;
        }

        if (*f == '%') {
            format_putc(&out, '%');
            continue;
        }

        int precision = -1;
        if (*f == '.') {
            precision = 0;
            ++f;

            if (*f == '*') {
                precision = va_arg(args, int);
                if (precision < 0) {
                    precision = -1;
                }
                ++f;
            } else {
                while (*f >= '0' && *f <= '9') {
                    precision = precision * 10 + (*f - '0');
                    ++f;
                }
            }
        }

        int long_long_mod = 0;
        int long_mod = 0;
        int size_mod = 0;

        if (*f == 'l') {
            ++f;
            if (*f == 'l') {
                long_long_mod = 1;
                ++f;
            } else {
                long_mod = 1;
            }
        } else if (*f == 'z') {
            size_mod = 1;
            ++f;
        }

        switch (*f) {
        case 's':
            format_puts_n(&out, va_arg(args, const char *), precision);
            break;
        case 'c':
            format_putc(&out, (char)va_arg(args, int));
            break;
        case 'p': {
            uintptr_t addr = (uintptr_t)va_arg(args, void *);
            format_puts_n(&out, "0x", -1);
            format_put_uint(&out, (unsigned long long)addr, 16, false);
            break;
        }
        case 'd':
        case 'i':
            if (long_long_mod) {
                format_put_int(&out, va_arg(args, long long));
            } else if (long_mod) {
                format_put_int(&out, (long long)va_arg(args, long));
            } else if (size_mod) {
                format_put_int(&out, (long long)va_arg(args, size_t));
            } else {
                format_put_int(&out, (long long)va_arg(args, int));
            }
            break;
        case 'u':
            if (long_long_mod) {
                format_put_uint(&out, va_arg(args, unsigned long long), 10, false);
            } else if (long_mod) {
                format_put_uint(&out, (unsigned long long)va_arg(args, unsigned long), 10, false);
            } else if (size_mod) {
                format_put_uint(&out, (unsigned long long)va_arg(args, size_t), 10, false);
            } else {
                format_put_uint(&out, (unsigned long long)va_arg(args, unsigned int), 10, false);
            }
            break;
        case 'x':
        case 'X': {
            bool uppercase = *f == 'X';
            if (long_long_mod) {
                format_put_uint(&out, va_arg(args, unsigned long long), 16, uppercase);
            } else if (long_mod) {
                format_put_uint(&out, (unsigned long long)va_arg(args, unsigned long), 16, uppercase);
            } else if (size_mod) {
                format_put_uint(&out, (unsigned long long)va_arg(args, size_t), 16, uppercase);
            } else {
                format_put_uint(&out, (unsigned long long)va_arg(args, unsigned int), 16, uppercase);
            }
            break;
        }
        default:
            format_putc(&out, '%');
            format_putc(&out, *f);
            break;
        }
    }

    if (s != NULL && n > 0) {
        size_t nul_index = out.len < n ? out.len : n - 1;
        s[nul_index] = '\0';
    }

    return (int)out.len;
}

int snprintf(char *s, size_t n, const char *format, ...)
{
    va_list args;
    va_start(args, format);
    int ret = vsnprintf(s, n, format, args);
    va_end(args);
    return ret;
}

char *format_alloc(const char *format_string, ...)
{
    va_list args;
    va_start(args, format_string);

    va_list args_copy;
    va_copy(args_copy, args);

    int needed = vsnprintf(NULL, 0, format_string, args);
    va_end(args);

    if (needed < 0) {
        va_end(args_copy);
        return NULL;
    }

    char *result = malloc((size_t)needed + 1);
    if (result == NULL) {
        va_end(args_copy);
        return NULL;
    }

    vsnprintf(result, (size_t)needed + 1, format_string, args_copy);
    va_end(args_copy);
    return result;
}

int printf(const char *restrict format, ...) {
    va_list args;
    va_start(args, format);
    int ret = vsay(format, args);
    va_end(args);
    return ret;
}
