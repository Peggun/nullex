/**
 * @file string.h
 * @brief String and memory manipulation functions for Nullex userspace.
 *
 * Provides common string-processing and memory-manipulation routines similar
 * to those found in the standard C library.
 */

#ifndef NULLEX_STRING_H
#define NULLEX_STRING_H

#include <stdbool.h>
#include <stddef.h>

/**
 * @brief Calculate the length of a null-terminated string.
 *
 * @param[in] str String whose length is to be calculated.
 * @return Number of characters in the string, excluding the terminating
 *         null character.
 */
size_t strlen(const char *str);

/**
 * @brief Calculate the length of a string up to a maximum number of characters.
 *
 * @param[in] s String whose length is to be calculated.
 * @param[in] maxlen Maximum number of characters to examine.
 * @return Number of characters before the terminating null character or
 *         @p maxlen, whichever occurs first.
 */
size_t strnlen(const char *s, size_t maxlen);

/**
 * @brief Compare two null-terminated strings.
 *
 * @param[in] str1 First string to compare.
 * @param[in] str2 Second string to compare.
 * @return A value less than zero if @p str1 is less than @p str2, zero if
 *         they are equal, or a value greater than zero if @p str1 is greater
 *         than @p str2.
 */
int strcmp(const char *str1, const char *str2);

/**
 * @brief Compare up to a specified number of characters from two strings.
 *
 * @param[in] s1 First string to compare.
 * @param[in] s2 Second string to compare.
 * @param[in] n Maximum number of characters to compare.
 * @return A value less than zero if @p s1 is less than @p s2, zero if they
 *         are equal for the compared characters, or a value greater than
 *         zero if @p s1 is greater than @p s2.
 */
int strncmp(const char *s1, const char *s2, register size_t n);

/**
 * @brief Copy a null-terminated string into a destination buffer.
 *
 * @param[out] dest Destination buffer.
 * @param[in] src Source string.
 * @return Pointer to the destination buffer.
 *
 * @warning The destination buffer must be large enough to contain the source
 *          string and its terminating null character.
 */
char *strcpy(char *__restrict dest, const char *__restrict src);

/**
 * @brief Split a string into a sequence of tokens.
 *
 * @param[in,out] s String to tokenize, or @c NULL to continue tokenization
 *                  of the previous string.
 * @param[in] delim String containing delimiter characters.
 * @return Pointer to the next token, or @c NULL if no token remains.
 */
char *strtok(char *s, const char *delim);

/**
 * @brief Reentrant version of @c strtok.
 *
 * @param[in,out] s String to tokenize, or @c NULL to continue tokenization.
 * @param[in] delim String containing delimiter characters.
 * @param[out] save_ptr Pointer to storage used to maintain tokenization state.
 * @return Pointer to the next token, or @c NULL if no token remains.
 */
char *strtok_r(char *s, const char *delim, char **save_ptr);

/**
 * @brief Calculate the length of the initial segment containing only
 *        characters from a specified set.
 *
 * @param[in] str String to examine.
 * @param[in] accept String containing the accepted characters.
 * @return Number of characters at the beginning of @p str that are present
 *         in @p accept.
 */
size_t strspn(const char *str, const char *accept);

/**
 * @brief Find the first character in a string that matches any character
 *        from a specified set.
 *
 * @param[in] s String to search.
 * @param[in] accept String containing characters to search for.
 * @return Pointer to the first matching character, or @c NULL if none is found.
 */
char *strpbrk(const char *s, const char *accept);

/**
 * @brief Calculate the length of the initial segment containing no characters
 *        from a specified set.
 *
 * @param[in] s1 String to examine.
 * @param[in] s2 String containing characters to reject.
 * @return Number of characters before the first character from @p s2 is
 *         encountered.
 */
size_t strcspn(const char *s1, const char *s2);

/**
 * @brief Copy up to a specified number of characters from one string to
 * another.
 *
 * @param[out] s1 Destination buffer.
 * @param[in] s2 Source string.
 * @param[in] n Maximum number of characters to copy.
 * @return Pointer to the destination buffer.
 *
 * @warning If @p s2 is shorter than @p n characters, the remainder of the
 *          destination buffer is padded with null bytes.
 */
char *strncpy(char *s1, const char *s2, size_t n);

/**
 * @brief Create a duplicate of a null-terminated string.
 *
 * @param[in] s String to duplicate.
 * @return Pointer to a newly allocated copy of the string, or @c NULL if
 *         allocation fails.
 *
 * @note The returned string must be released using the appropriate Nullex
 *       memory-management function.
 */
char *strdup(const char *s);

/**
 * @brief Find the first occurrence of a character in a string.
 *
 * @param[in] s String to search.
 * @param[in] c_in Character to locate.
 * @return Pointer to the first occurrence of the character, or @c NULL if
 *         it is not found.
 */
char *strchr(const char *s, int c_in);

/**
 * @brief Find the last occurrence of a character in a string.
 *
 * @param[in] s String to search.
 * @param[in] c Character to locate.
 * @return Pointer to the last occurrence of the character, or @c NULL if
 *         it is not found.
 */
char *strrchr(const char *s, int c);

/**
 * @brief Find the first occurrence of a substring.
 *
 * @param[in] s1 String to search.
 * @param[in] s2 Substring to locate.
 * @return Pointer to the first occurrence of @p s2 in @p s1, or @c NULL if
 *         the substring is not found.
 */
char *strstr(const char *s1, const char *s2);

/**
 * @brief Append one null-terminated string to another.
 *
 * @param[in,out] dest Destination string.
 * @param[in] src String to append.
 * @return Pointer to the destination string.
 *
 * @warning The destination buffer must have enough free space to contain
 *          the original string, appended string, and terminating null byte.
 */
char *strcat(char *dest, const char *src);

/**
 * @brief Copy a block of memory from one location to another.
 *
 * @param[out] dest Destination memory region.
 * @param[in] src Source memory region.
 * @param[in] len Number of bytes to copy.
 * @return Pointer to the destination memory region.
 *
 * @warning The source and destination regions must not overlap.
 */
void *memcpy(void *dest, const void *src, size_t len);

/**
 * @brief Fill a block of memory with a specified byte value.
 *
 * @param[out] dest Memory region to fill.
 * @param[in] val Byte value to write.
 * @param[in] len Number of bytes to write.
 * @return Pointer to the destination memory region.
 */
void *memset(void *dest, int val, size_t len);

/**
 * @brief Search a block of memory for a specified byte.
 *
 * @param[in] src_void Memory region to search.
 * @param[in] c Byte value to search for.
 * @param[in] length Number of bytes to examine.
 * @return Pointer to the first matching byte, or @c NULL if no match is found.
 */
void *memchr(register const void *src_void, int c, size_t length);

/**
 * @brief Compare two memory regions for equality.
 *
 * @param[in] s1 First memory region to compare.
 * @param[in] s2 Second memory region to compare.
 * @param[in] n Number of bytes to compare.
 * @return 0 if the regions are equal, a negative value if s1 is less than s2,
 *         or a positive value if s1 is greater than s2.
 */
int memcmp(const void *s1, const void *s2, size_t n);

/**
 * @brief Remove a trailing newline character from a string.
 *
 * @param[in,out] s String to modify.
 */
static void trim_newline(char *s);

/**
 * @brief Read a line of input into a buffer.
 *
 * Displays a prompt and reads a line of input into the supplied buffer.
 *
 * @param[in] prompt Prompt displayed before reading input.
 * @param[out] buffer Buffer in which to store the input.
 * @param[in] len Maximum size of the destination buffer.
 */
static void read_line(const char *prompt, char *buffer, size_t len);

/**
 * @brief Check whether a string ends with a specified suffix.
 *
 * @param[in] string String to examine.
 * @param[in] end Suffix to search for.
 * @return @c true if @p string ends with @p end, otherwise @c false.
 */
bool endswith(char *string, char *end);

/**
 * @brief Check whether a string begins with a specified prefix.
 *
 * @param[in] string String to examine.
 * @param[in] start Prefix to search for.
 * @return @c true if @p string begins with @p start, otherwise @c false.
 */
bool startswith(char *string, char *start);

#endif