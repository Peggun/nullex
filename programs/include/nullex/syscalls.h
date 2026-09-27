/**
 * @file syscalls.h
 * @brief Nullex userspace system call interface.
 *
 * Provides syscall numbers, the low-level syscall invocation mechanism,
 * formatted output support, and convenient wrappers for invoking Nullex
 * kernel services from userspace.
 *
 * System calls are issued using the x86_64 @c int $0x80 instruction.
 */

#ifndef NULLEX_SYSCALLS_H
#define NULLEX_SYSCALLS_H

#include <nullex/sysutils.h>
#include <stdarg.h>
#include <stddef.h>
#include <stdint.h>

/**
 * @name System Call Numbers
 * @{
 */

/** @brief Write formatted output to standard output. */
#define SYS_SAY 0

/** @brief Terminate the current process with an exit code. */
#define SYS_HALT 1

/** @brief Split execution into a separate process or execution context. */
#define SYS_SPLIT 2

/** @brief Wait for the current process or task. */
#define SYS_WAITON 3

/** @brief Open a filesystem object. */
#define SYS_OPENF 4

/** @brief Close a filesystem file descriptor. */
#define SYS_CLOSEF 5

/** @brief Read data from a filesystem file descriptor. */
#define SYS_READF 6

/** @brief Write data to a filesystem file descriptor. */
#define SYS_WRITEF 7

/** @brief Open a directory. */
#define SYS_OPEND 8

/** @brief Close a directory descriptor. */
#define SYS_CLOSED 9

/** @brief Execute a program. */
#define SYS_RUN 10

/** @brief Stop a process. */
#define SYS_STOP 11

/** @brief Suspend execution for a period of time. */
#define SYS_NAP 12

/** @brief Obtain the size of a file. */
#define SYS_SIZEF 13

/** @brief Create a socket. */
#define SYS_CSOCKET 14

/** @brief Connect a socket to a remote host. */
#define SYS_CONNSOCK 15

/** @brief Send data through a socket. */
#define SYS_SEND 16

/** @brief Receive data from a socket. */
#define SYS_RECV 17

/** @brief Close a socket. */
#define SYS_CLOSESOCK 18

/** @brief Retrieve directory entry information. */
#define SYS_GETDIRENTS 19

/** @brief Spawn a new process. */
#define SYS_SPAWNP 20

/** @brief Remove a file. */
#define SYS_RMFILE 21

/** @brief Remove a directory. */
#define SYS_RMDIR 22

/** @} */

/**
 * @brief Invoke a Nullex system call.
 *
 * Places the syscall number and arguments into the x86_64 calling convention
 * registers and invokes the kernel through software interrupt @c 0x80.
 *
 * @param[in] num System call number.
 * @param[in] a0 First system call argument.
 * @param[in] a1 Second system call argument.
 * @param[in] a2 Third system call argument.
 * @param[in] a3 Fourth system call argument.
 * @param[in] a4 Fifth system call argument.
 * @param[in] a5 Sixth system call argument.
 * @return The 32-bit result returned by the kernel.
 */
static inline int32_t ksyscall(uint32_t num, uint64_t a0, uint64_t a1,
                               uint64_t a2, uint64_t a3, uint64_t a4,
                               uint64_t a5) {
  int32_t ret;
  register uint64_t r10 __asm__("r10") = a3;
  register uint64_t r8 __asm__("r8") = a4;
  register uint64_t r9 __asm__("r9") = a5;

  __asm__ volatile("int $0x80"
                   : "=a"(ret)
                   : "a"(num), "D"(a0), "S"(a1), "d"(a2), "r"(r10), "r"(r8),
                     "r"(r9)
                   : "rcx", "r11", "memory");

  return ret;
}

/**
 * @brief Format and print output using the Nullex syscall interface.
 *
 * Supports a subset of printf-style formatting and writes the resulting
 * string to the kernel using @c SYS_SAY.
 *
 * @param[in] format Format string.
 * @param[in,out] args Variable argument list referenced by @p format.
 * @return Number of bytes reported by the underlying syscall.
 */
static int32_t vsay(const char *format, va_list args) {
  char buf[1024];
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

    int zero_pad = 0;
    int width = 0;
    int alt_form = 0;
    int left_align = 0;

    while (*f == '0' || *f == '#' || *f == '-') {
      if (*f == '0') {
        zero_pad = 1;
      } else if (*f == '#') {
        alt_form = 1;
      } else if (*f == '-') {
        left_align = 1;
        zero_pad = 0;
      }
      f++;
    }

    while (*f >= '0' && *f <= '9') {
      width = width * 10 + (*f - '0');
      f++;
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
      } else {
        while (*f >= '0' && *f <= '9') {
          precision_from_arg = precision_from_arg * 10 + (*f - '0');
          has_precision_from_arg = 1;
          f++;
        }
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

      size_t str_len = 0;
      if (has_precision_from_arg) {
        while (str_len < (size_t)precision_from_arg && str[str_len] != '\0') {
          str_len++;
        }
      } else {
        while (str[str_len] != '\0') {
          str_len++;
        }
      }

      size_t padding = 0;
      if (width > (int)str_len) {
        padding = width - str_len;
      }

      if (!left_align) {
        for (size_t i = 0; i < padding && p < end; i++) {
          *p++ = ' ';
        }
      }

      for (size_t i = 0; i < str_len && p < end; i++) {
        *p++ = str[i];
      }

      if (left_align) {
        for (size_t i = 0; i < padding && p < end; i++) {
          *p++ = ' ';
        }
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
      say_append_uint_fmt(&p, (unsigned long long)addr, 16, sizeof(void *) * 2,
                          1, 0, 0, end);

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
      unsigned long long num;

      if (long_long)
        num = va_arg(args, unsigned long long);
      else if (long_mod)
        num = va_arg(args, unsigned long);
      else if (size_mod)
        num = va_arg(args, size_t);
      else
        num = va_arg(args, unsigned int);

      if (width)
        say_append_uint_fmt(&p, num, 10, width, zero_pad, 0, 0, end);
      else
        say_append_uint(&p, num, 10, end);

      f++;
      continue;
    }

    if (*f == 'x' || *f == 'X') {
      unsigned long long num;

      if (long_long)
        num = va_arg(args, unsigned long long);
      else if (long_mod)
        num = va_arg(args, unsigned long);
      else if (size_mod)
        num = va_arg(args, size_t);
      else
        num = va_arg(args, unsigned int);

      say_append_uint_fmt(&p, num, 16, width, zero_pad, alt_form, *f == 'X',
                          end);

      f++;
      continue;
    }

    *p++ = '%';

    if (p < end && *f) *p++ = *f++;
  }

  *p = '\0';

  size_t len = (size_t)(p - buf);

  return ksyscall(SYS_SAY, (uint64_t)buf, (uint64_t)len, 0, 0, 0, 0);
}

/**
 * @brief Print formatted output to the Nullex standard output.
 *
 * Formats the supplied arguments and writes the resulting string through
 * the @c SYS_SAY system call.
 *
 * @param[in] format Format string.
 * @param[in] ... Arguments referenced by @p format.
 * @return Result returned by the underlying @c SYS_SAY syscall.
 */
static inline int32_t say(const char *format, ...) {
  va_list args;
  va_start(args, format);

  int32_t ret = vsay(format, args);

  va_end(args);
  return ret;
}

/**
 * @brief Terminate the current process.
 *
 * @param[in] exit_code Process exit status.
 * @return Result returned by the @c SYS_HALT syscall.
 */
static inline int32_t halt(int64_t exit_code) {
  return ksyscall(SYS_HALT, (uint64_t)exit_code, 0, 0, 0, 0, 0);
}

/**
 * @brief Split the current execution context.
 *
 * @return Result returned by the @c SYS_SPLIT syscall.
 */
static inline int32_t split(void) {
  return ksyscall(SYS_SPLIT, 0, 0, 0, 0, 0, 0);
}

/**
 * @brief Wait for the current process or task.
 *
 * @return Result returned by the @c SYS_WAITON syscall.
 */
static inline int32_t waiton(void) {
  return ksyscall(SYS_WAITON, 0, 0, 0, 0, 0, 0);
}

/**
 * @brief Execute a program from a specified path.
 *
 * @param[in] path Path to the executable.
 * @param[in] len Length of @p path in bytes.
 * @return Process identifier or a negative error code.
 */
static inline int32_t run(const char *path, unsigned len) {
  return ksyscall(SYS_RUN, (uint64_t)path, (uint64_t)len, 0, 0, 0, 0);
}

/**
 * @brief Stop a process.
 *
 * @param[in] pid Process identifier.
 * @return Result returned by the @c SYS_STOP syscall.
 */
static inline int32_t stop(uint64_t pid) {
  return ksyscall(SYS_STOP, pid, 0, 0, 0, 0, 0);
}

/**
 * @brief Suspend execution for a period of time.
 *
 * @return Result returned by the @c SYS_NAP syscall.
 */
static inline int32_t nap(void) { return ksyscall(SYS_NAP, 0, 0, 0, 0, 0, 0); }

#endif