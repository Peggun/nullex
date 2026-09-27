/**
 * @file ctype.h
 * @brief Character and string case-conversion utilities.
 */

#ifndef NULLEX_CTYPE_H
#define NULLEX_CTYPE_H

/**
 * @brief Convert an uppercase character to lowercase.
 *
 * If @p ch is an uppercase alphabetic character, its lowercase equivalent
 * is returned. Characters that are not uppercase alphabetic characters are
 * returned unchanged.
 *
 * @param[in] ch Character to convert.
 * @return Lowercase equivalent of @p ch, or @p ch unchanged if it is not
 *         an uppercase alphabetic character.
 */
char toLower(char ch);

/**
 * @brief Convert a string to lowercase.
 *
 * Converts all uppercase alphabetic characters in the supplied string to
 * their lowercase equivalents. Other characters are left unchanged.
 *
 * @param[in,out] str Null-terminated string to convert.
 */
void strToLower(char *str);

#endif