/**
 * @file sysutils.h
 * @brief Internal formatting and output helper utilities.
 *
 * Provides small inline helper functions used by the Nullex userspace
 * syscall and formatted-output interfaces.
 *
 * These functions operate on caller-provided output buffers and provide
 * support for appending characters, strings, and formatted integer values.
 */

#ifndef NULLEX_SYSUTILS_H
#define NULLEX_SYSUTILS_H

/**
 * @brief Append a single character to an output buffer.
 *
 * Writes @p c at the current output position and advances the position
 * when sufficient space remains.
 *
 * @param[in,out] p Pointer to the current output position.
 * @param[in] c Character to append.
 * @param[in] end Pointer one byte past the last writable output position.
 */
static inline void say_append_char(char **p, char c, char *end) {
  if (*p < end) {
    **p = c;
    (*p)++;
  }
}

/**
 * @brief Append a null-terminated string to an output buffer.
 *
 * Copies characters from @p s into the output buffer until the end of the
 * string or the end of the available buffer is reached.
 *
 * @param[in,out] p Pointer to the current output position.
 * @param[in] s Null-terminated string to append.
 * @param[in] end Pointer one byte past the last writable output position.
 */
static inline void say_append_str(char **p, const char *s, char *end) {
  while (*s && *p < end) {
    **p = *s;
    (*p)++;
    s++;
  }
}

/**
 * @brief Append up to a specified number of characters to an output buffer.
 *
 * Stops when @p n characters have been written, a null character is
 * encountered, or the output buffer is full.
 *
 * @param[in,out] p Pointer to the current output position.
 * @param[in] s Source string.
 * @param[in] n Maximum number of characters to append.
 * @param[in] end Pointer one byte past the last writable output position.
 */
static inline void say_append_strn(char **p, const char *s, int n, char *end) {
  for (int i = 0; i < n && s[i] && *p < end; i++) {
    **p = s[i];
    (*p)++;
  }
}

/**
 * @brief Append an unsigned integer in a specified base.
 *
 * Converts @p num to its textual representation using @p base and appends
 * the resulting characters to the output buffer.
 *
 * @param[in,out] p Pointer to the current output position.
 * @param[in] num Unsigned integer to format.
 * @param[in] base Numeric base used for conversion.
 * @param[in] end Pointer one byte past the last writable output position.
 *
 * @note Bases greater than 10 use lowercase hexadecimal-style characters
 *       for digits above 9.
 */
static inline void say_append_uint(char **p, unsigned long long num,
                                   unsigned base, char *end) {
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

/**
 * @brief Append a signed integer in a specified base.
 *
 * Converts @p num to its textual representation and appends the result
 * to the output buffer. Negative values are prefixed with @c '-'.
 *
 * @param[in,out] p Pointer to the current output position.
 * @param[in] num Signed integer to format.
 * @param[in] base Numeric base used for conversion.
 * @param[in] end Pointer one byte past the last writable output position.
 */
static inline void say_append_int(char **p, long long num, unsigned base,
                                  char *end) {
  if (num < 0) {
    say_append_char(p, '-', end);
    num = -num;
  }

  say_append_uint(p, (unsigned long long)num, base, end);
}

/**
 * @brief Append a formatted unsigned integer to an output buffer.
 *
 * Supports field width, zero-padding, hexadecimal alternate prefixes,
 * and uppercase hexadecimal digits.
 *
 * @param[in,out] p Pointer to the current output position.
 * @param[in] value Unsigned integer to format.
 * @param[in] base Numeric base used for conversion.
 * @param[in] width Minimum field width.
 * @param[in] zero_pad Whether to pad the field with zeroes instead of spaces.
 * @param[in] alt_form Whether to use an alternate representation such as
 *                     the @c 0x hexadecimal prefix.
 * @param[in] uppercase Whether alphabetic digits and prefixes should be
 *                       uppercase.
 * @param[in] end Pointer one byte past the last writable output position.
 *
 * @note When @p base is 16 and @p alt_form is enabled, non-zero values are
 *       prefixed with @c 0x or @c 0X depending on @p uppercase.
 */
static inline void say_append_uint_fmt(char **p, unsigned long long value,
                                       unsigned base, int width, int zero_pad,
                                       int alt_form, int uppercase, char *end) {
  char tmp[65];
  int pos = 0;

  do {
    unsigned digit = value % base;

    if (digit < 10)
      tmp[pos++] = '0' + digit;
    else
      tmp[pos++] = (uppercase ? 'A' : 'a') + (digit - 10);

    value /= base;
  } while (value && pos < (int)sizeof(tmp));

  int is_zero = (pos == 1 && tmp[0] == '0');
  int show_prefix = (alt_form && base == 16 && !is_zero);

  if (show_prefix) {
    width -= 2;
  }

  if (show_prefix && zero_pad) {
    say_append_str(p, uppercase ? "0X" : "0x", end);
    show_prefix = 0;
  }

  while (pos < width && *p < end) {
    *(*p)++ = zero_pad ? '0' : ' ';
    width--;
  }

  if (show_prefix) {
    say_append_str(p, uppercase ? "0X" : "0x", end);
  }

  while (pos > 0 && *p < end) *(*p)++ = tmp[--pos];
}

#endif