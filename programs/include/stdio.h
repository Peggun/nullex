/**
 * @file stdio.h
 * @brief Formatted input and output utilities.
 *
 * Provides functions for formatted output and string formatting,
 * including support for variable argument lists.
 */

#ifndef NULLEX_STDIO_H
#define NULLEX_STDIO_H

#include <stdarg.h>
#include <stddef.h>

/**
 * @brief Print formatted output.
 *
 * Formats the supplied arguments according to @p format and writes the
 * resulting output to the standard output stream.
 *
 * @param[in] format Format string describing the output.
 * @param[in] ... Arguments referenced by @p format.
 * @return Number of characters written, or a negative value on failure.
 */
int printf(const char *restrict format, ...);

/**
 * @brief Format a string into a fixed-size buffer using a variable argument
 * list.
 *
 * Formats the supplied arguments according to @p format and writes the
 * result to @p s, writing at most @p n bytes including the terminating
 * null character.
 *
 * @param[out] s Destination buffer.
 * @param[in] n Size of the destination buffer in bytes.
 * @param[in] format Format string describing the output.
 * @param[in,out] args Variable argument list containing the format arguments.
 * @return The number of characters that would have been written excluding
 *         the terminating null character.
 */
int vsnprintf(char *s, size_t n, const char *format, va_list args);

/**
 * @brief Format a string into a fixed-size buffer.
 *
 * Formats the supplied arguments according to @p format and writes the
 * result to @p s, writing at most @p n bytes including the terminating
 * null character.
 *
 * @param[out] s Destination buffer.
 * @param[in] n Size of the destination buffer in bytes.
 * @param[in] format Format string describing the output.
 * @param[in] ... Arguments referenced by @p format.
 * @return The number of characters that would have been written excluding
 *         the terminating null character.
 */
int snprintf(char *s, size_t n, const char *format, ...);

#endif