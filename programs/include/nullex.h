/*

    nullex.h

    Nullex userspace definitions. Slowly expand this to allow for better
    userspace programs for the kernel.

*/

#pragma once

typedef unsigned long      size_t;

typedef unsigned char      uint8_t;
typedef unsigned short     uint16_t;
typedef unsigned int       uint32_t;
typedef unsigned long long uint64_t;

typedef signed char      int8_t;
typedef signed short     int16_t;
typedef signed int       int32_t;
typedef signed long long int64_t;

#define SYS_SAY    0
#define SYS_HALT   1
#define SYS_SPLIT  2
#define SYS_WAITON 3
#define SYS_OPENF  4
#define SYS_CLOSEF 5
#define SYS_READF  6
#define SYS_WRITEF 7
#define SYS_RUN    8
#define SYS_STOP   9
#define SYS_NAP    10
#define SYS_SIZEF  11

/*
 * x86_64 syscall wrapper using the Linux-style syscall register convention:
 * rax = syscall number (also return)
 * rdi = arg0
 * rsi = arg1
 * rdx = arg2
 * r10 = arg3
 * r8  = arg4
 * r9  = arg5
 *
 * Clobbers: rcx, r11, memory
 */
static inline int32_t ksyscall(uint32_t num, uint64_t a0, uint64_t a1, uint64_t a2, uint64_t a3, uint64_t a4, uint64_t a5) {
    int32_t ret;
    register uint64_t r10 __asm__("r10") = a3; // r10 is special, can't be a constraint
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

static inline size_t strlen(const char *str) {
    size_t length = 0;
    while (str[length] != '\0') {
        length++;
    }
    return length;
}

static inline char* strcpy(char* destination, const char* source) {
    char *start = destination;
    while (*source) {
        *destination++ = *source++;
    }
    *destination = 0; 
    return start;
}

// ai thanks.
static inline int32_t say(const char* format, ...) {
    char buf[256];
    char* p = buf;
    const char* f = format;
    
    __builtin_va_list args;
    __builtin_va_start(args, format);
    
    while (*f && p - buf < 255) {
        if (*f == '%') {
            f++;
            if (*f == 'd' || *f == 'i') {
                long num = __builtin_va_arg(args, long);
                char nbuf[32];
                int neg = (num < 0);
                if (neg) num = -num;
                
                int i = 0;
                if (num == 0) {
                    nbuf[i++] = '0';
                } else {
                    while (num > 0) {
                        nbuf[i++] = '0' + (num % 10);
                        num /= 10;
                    }
                }
                if (neg) nbuf[i++] = '-';
                
                for (int j = i - 1; j >= 0; j--) {
                    *p++ = nbuf[j];
                }
            } else if (*f == 's') {
                const char* str = __builtin_va_arg(args, const char*);
                while (*str && p - buf < 255) {
                    *p++ = *str++;
                }
            } else if (*f == 'c') {
                char c = __builtin_va_arg(args, int);
                *p++ = c;
            } else if (*f == 'x') {
                long num = __builtin_va_arg(args, long);
                char nbuf[32];
                int i = 0;
                if (num == 0) {
                    nbuf[i++] = '0';
                } else {
                    while (num > 0) {
                        int digit = num % 16;
                        nbuf[i++] = (digit < 10) ? ('0' + digit) : ('a' + digit - 10);
                        num /= 16;
                    }
                }
                for (int j = i - 1; j >= 0; j--) {
                    *p++ = nbuf[j];
                }
            }
            f++;
        } else {
            *p++ = *f++;
        }
    }
    *p = '\0';
    __builtin_va_end(args);
    
    size_t len = p - buf;
    return ksyscall(SYS_SAY, (uint64_t)buf, (uint64_t)len, 0, 0, 0, 0);
}

// SYS_HALT is in _start.c as _exit()
static inline int32_t halt(int64_t exit_code) {
    return ksyscall(SYS_HALT, (uint64_t)exit_code, 0, 0, 0, 0, 0);
}

static inline int32_t split() {
    return ksyscall(SYS_SPLIT, 0, 0, 0, 0, 0, 0);
}

static inline int32_t waiton() {
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
    char destination_array[len];

    strcpy(destination_array, to_write);

    uint8_t *buf_ptr = (uint8_t *)destination_array;

    return ksyscall(SYS_WRITEF, fd, (uint64_t)buf_ptr, (uint64_t)len, 0, 0, 0);
}

#define writef(fd, arg) _Generic((arg), \
    const char*: writef_str,            \
    char*:       writef_str,            \
    uint8_t*:    writef_buf,            \
    const uint8_t*: writef_buf          \
)(fd, arg)

static inline int32_t run(const char* path, unsigned len) {
    return ksyscall(SYS_RUN, (uint64_t)path, (uint64_t)len, 0, 0, 0, 0);
}

static inline int32_t stop(uint64_t pid) {
    return ksyscall(SYS_STOP, pid, 0, 0, 0, 0, 0);
}

static inline int32_t nap(/* todo */) {
    return ksyscall(SYS_NAP, 0, 0, 0, 0, 0, 0);
}

static inline int32_t sizef(uint64_t fd) {
    return ksyscall(SYS_SIZEF, fd, 0, 0, 0, 0, 0);
}   