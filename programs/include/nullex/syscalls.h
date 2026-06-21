// programs/include/nullex/syscalls.h
#ifndef NULLEX_SYSCALLS_H
#define NULLEX_SYSCALLS_H

#include "../stdint.h"
#include "../stddef.h"
#include "../string.h"
#include "../stdarg.h"

#include "dirent.h"
#include "sysutils.h"

#define SYS_SAY        0
#define SYS_HALT       1
#define SYS_SPLIT      2
#define SYS_WAITON     3
#define SYS_OPENF      4
#define SYS_CLOSEF     5
#define SYS_READF      6
#define SYS_WRITEF     7
#define SYS_OPEND      8
#define SYS_RUN        9
#define SYS_STOP       10
#define SYS_NAP        11
#define SYS_SIZEF      12
#define SYS_CSOCKET    13
#define SYS_CONNSOCK   14
#define SYS_SEND       15
#define SYS_RECV       16
#define SYS_CLOSESOCK  17
#define SYS_GETDIRENTS 18

#if defined(__x86_64__) || defined(_M_X64)
static inline int32_t ksyscall(uint32_t num, uint64_t a0, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5) {
    int32_t ret;
    register uint64_t r10 __asm__("r10") = a3;
    register uint64_t r8  __asm__("r8")  = a4;
    register uint64_t r9  __asm__("r9")  = a5;

    __asm__ volatile (
        "int $0x80"
        : "=a"(ret)
        : "a"(num), "D"(a0), "S"(a1), "d"(a2), "r"(r10), "r"(r8), "r"(r9)
        : "rcx", "r11", "memory"
    );

    return ret;
}
#endif

#if defined(__aarch64__) || defined(_M_ARM64)
static inline int32_t ksyscall(uint32_t num, uint64_t a0, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5) {
    int32_t ret;
    register uint64_t x8 __asm__("x8") = num;
    register uint64_t x0 __asm__("x0") = a0;
    register uint64_t x1 __asm__("x1") = a1;
    register uint64_t x2 __asm__("x2") = a2;
    register uint64_t x3 __asm__("x3") = a3;
    register uint64_t x4 __asm__("x4") = a4;
    register uint64_t x5 __asm__("x5") = a5;

    __asm__ volatile (
        "svc #0"
        : "=r"(ret)
        : "r"(x8), "r"(x0), "r"(x1), "r"(x2), "r"(x3), "r"(x4), "r"(x5)
        : "memory"
    );

    return ret;
}
#endif

static int32_t vsay(const char *format, va_list args) {
    char buf[256];
    char *p = buf;
    char *end = buf + sizeof(buf) - 1;
    const char *f = format;

    while (*f && p < end) {
        if (*f != '%') {
            *p++ = *f++;
            continue;
        }

        f++;

        if (*f == '%') {
            *p++ = '%';
            f++;
            continue;
        }

        int long_long = 0;
        int long_mod = 0;
        int size_mod = 0;
        int precision_from_arg = 0;
        int has_precision_from_arg = 0;

        if (*f == '.') {
            f++;
            if (*f == '*') {
                precision_from_arg = va_arg(args, int);
                has_precision_from_arg = 1;
                f++;
            }
        }

        if (*f == 'l') {
            f++;
            if (*f == 'l') {
                long_long = 1;
                f++;
            } else {
                long_mod = 1;
            }
        } else if (*f == 'z') {
            size_mod = 1;
            f++;
        }

        if (*f == 's') {
            const char *str = va_arg(args, const char *);
            if (!str) str = "(null)";

            if (has_precision_from_arg) {
                say_append_strn(&p, str, precision_from_arg, end);
            } else {
                say_append_str(&p, str, end);
            }
            f++;
            continue;
        }

        if (*f == 'c') {
            char c = (char)va_arg(args, int);
            say_append_char(&p, c, end);
            f++;
            continue;
        }

        if (*f == 'p') {
            void *ptr = va_arg(args, void *);
            uintptr_t addr = (uintptr_t)ptr;

            say_append_str(&p, "0x", end);
            say_append_uint(&p, (unsigned long long)addr, 16, end);

            f++;
            continue;
        }

        if (*f == 'd' || *f == 'i') {
            if (long_long) {
                long long num = va_arg(args, long long);
                say_append_int(&p, num, 10, end);
            } else if (long_mod) {
                long num = va_arg(args, long);
                say_append_int(&p, (long long)num, 10, end);
            } else {
                int num = va_arg(args, int);
                say_append_int(&p, (long long)num, 10, end);
            }
            f++;
            continue;
        }

        if (*f == 'u') {
            if (long_long) {
                unsigned long long num = va_arg(args, unsigned long long);
                say_append_uint(&p, num, 10, end);
            } else if (long_mod) {
                unsigned long num = va_arg(args, unsigned long);
                say_append_uint(&p, (unsigned long long)num, 10, end);
            } else if (size_mod) {
                size_t num = va_arg(args, size_t);
                say_append_uint(&p, (unsigned long long)num, 10, end);
            } else {
                unsigned int num = va_arg(args, unsigned int);
                say_append_uint(&p, (unsigned long long)num, 10, end);
            }
            f++;
            continue;
        }

        if (*f == 'x') {
            if (long_long) {
                unsigned long long num = va_arg(args, unsigned long long);
                say_append_uint(&p, num, 16, end);
            } else if (long_mod) {
                unsigned long num = va_arg(args, unsigned long);
                say_append_uint(&p, (unsigned long long)num, 16, end);
            } else if (size_mod) {
                size_t num = va_arg(args, size_t);
                say_append_uint(&p, (unsigned long long)num, 16, end);
            } else {
                unsigned int num = va_arg(args, unsigned int);
                say_append_uint(&p, (unsigned long long)num, 16, end);
            }
            f++;
            continue;
        }

        *p++ = '%';
        if (p < end && *f) {
            *p++ = *f++;
        }
    }

    *p = '\0';

    size_t len = (size_t)(p - buf);
    return ksyscall(SYS_SAY, (uint64_t)buf, (uint64_t)len, 0, 0, 0, 0);
}

static inline int32_t say(const char *format, ...) {
    va_list args;
    va_start(args, format);
    int32_t ret = vsay(format, args);
    va_end(args);
    return ret;
}

static inline int32_t halt(int64_t exit_code) {
    return ksyscall(SYS_HALT, (uint64_t)exit_code, 0, 0, 0, 0, 0);
}

static inline int32_t split(void) {
    return ksyscall(SYS_SPLIT, 0, 0, 0, 0, 0, 0);
}

static inline int32_t waiton(void) {
    return ksyscall(SYS_WAITON, 0, 0, 0, 0, 0, 0);
}

static inline int32_t openf(const char* path) {
    size_t len = strlen(path);
    return ksyscall(SYS_OPENF, (uint64_t)path, (uint64_t)len, 0, 0, 0, 0);
}

static inline int32_t closef(uint64_t fd) {
    return ksyscall(SYS_CLOSEF, fd, 0, 0, 0, 0, 0);
}

static inline int32_t readf(uint64_t fd, uint8_t* buf, size_t len) {
    return ksyscall(SYS_READF, fd, (uint64_t)buf, (uint64_t)len, 0, 0, 0);
}

static inline int32_t writef_buf(uint64_t fd, uint8_t* buf, size_t len) {
    return ksyscall(SYS_WRITEF, fd, (uint64_t)buf, (uint64_t)len, 0, 0, 0);
}

static inline int32_t writef_str(uint64_t fd, const char* to_write) {
    size_t len = strlen(to_write);
    char destination_array[len + 1];
    strcpy(destination_array, to_write);

    return ksyscall(SYS_WRITEF, fd, (uint64_t)(uint8_t*)destination_array, (uint64_t)len, 0, 0, 0);
}

#define writef(fd, arg) _Generic((arg), \
    const char*: writef_str,            \
    char*:       writef_str,            \
    uint8_t*:    writef_buf,             \
    const uint8_t*: writef_buf          \
)(fd, arg)

static inline int32_t opend(const char *path) {
    size_t len = strlen(path);
    return ksyscall(SYS_OPEND, (uint64_t)path, (uint64_t)len, 0, 0, 0, 0);
}

static inline int32_t run(const char* path, unsigned len) {
    return ksyscall(SYS_RUN, (uint64_t)path, (uint64_t)len, 0, 0, 0, 0);
}

static inline int32_t stop(uint64_t pid) {
    return ksyscall(SYS_STOP, pid, 0, 0, 0, 0, 0);
}

static inline int32_t nap(void) {
    return ksyscall(SYS_NAP, 0, 0, 0, 0, 0, 0);
}

static inline int32_t sizef(uint64_t fd) {
    return ksyscall(SYS_SIZEF, fd, 0, 0, 0, 0, 0);
}

static inline int32_t getdirents(uint64_t fd, DirEntryInfo *dei, uintptr_t out_cap) {
    return ksyscall(SYS_GETDIRENTS, fd, (uint64_t)dei, (uint64_t)out_cap, 0, 0, 0);
}

#endif