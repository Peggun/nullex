/**
 * @file stdint.h
 * @brief Fixed-width integer type definitions.
 *
 * Provides signed and unsigned integer types with explicitly defined
 * widths, along with an unsigned integer type capable of storing a
 * pointer value.
 */

#ifndef NULLEX_STDINT_H
#define NULLEX_STDINT_H

/**
 * @brief Signed 8-bit integer type.
 */
typedef signed char int8_t;

/**
 * @brief Signed 16-bit integer type.
 */
typedef short int int16_t;

/**
 * @brief Signed 32-bit integer type.
 */
typedef int int32_t;

/**
 * @brief Signed 64-bit integer type.
 */
typedef long long int int64_t;

/**
 * @brief Unsigned 8-bit integer type.
 */
typedef unsigned char uint8_t;

/**
 * @brief Unsigned 16-bit integer type.
 */
typedef unsigned short int uint16_t;

/**
 * @brief Unsigned 32-bit integer type.
 */
typedef unsigned int uint32_t;

/**
 * @brief Unsigned 64-bit integer type.
 */
typedef unsigned long long int uint64_t;

/**
 * @brief Unsigned integer type capable of storing a pointer value.
 *
 * The underlying type is provided by the compiler through
 * @c __UINTPTR_TYPE__ and is expected to be 64 bits on the
 * x86_64 default target.
 */
typedef __UINTPTR_TYPE__ uintptr_t;

#endif