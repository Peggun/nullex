/**
 * @file io.h
 * @brief Basic userspace input and output utilities.
 *
 * Provides convenience functions for reading input from standard input
 * into a caller-provided buffer.
 */

#ifndef NULLEX_IO_H
#define NULLEX_IO_H

#include <nullex/fs.h>
#include <stddef.h>
#include <stdint.h>

/**
 * @brief Read a line of input from standard input.
 *
 * Displays a prompt using @c say() and reads input from file descriptor 0
 * (standard input) into the supplied buffer. The resulting string is always
 * null-terminated when the buffer length is greater than zero.
 *
 * @param[in] msg Prompt displayed before reading input.
 * @param[out] buffer Buffer into which the input is written.
 * @param[in] len Size of the supplied buffer in bytes.
 * @return Number of bytes read, excluding the terminating null character.
 *         Returns @c 0 if @p len is zero or if the underlying read operation
 *         fails.
 *
 * @note At most @p len - 1 bytes are read to reserve space for the
 *       terminating null character.
 */
static inline int32_t input(const char *msg, char *buffer, size_t len) {
  say("%s", msg);

  if (len == 0) {
    return 0;
  }

  int32_t bytes_read = readf(0, (uint8_t *)buffer, len - 1);
  if (bytes_read < 0) {
    bytes_read = 0;
  }

  buffer[bytes_read] = '\0';
  return bytes_read;
}

#endif