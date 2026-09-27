/**
 * @file stddef.h
 * @brief Common definitions for object sizes and null pointers.
 *
 * Provides the @c NULL pointer constant and the @c size_t type used for
 * representing object and memory sizes.
 */

#ifndef NULLEX_STDDEF_H
#define NULLEX_STDDEF_H

/**
 * @brief Null pointer constant.
 *
 * Represents a pointer that does not refer to a valid object or function.
 */
#define NULL ((void *)0)

/**
 * @brief Unsigned integer type used to represent object sizes.
 *
 * The underlying type is provided by the compiler through
 * @c __SIZE_TYPE__.
 */
typedef __SIZE_TYPE__ size_t;

#endif