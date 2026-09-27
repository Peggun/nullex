/**
 * @file pointer_arith.h
 * @brief Pointer and integer alignment utilities.
 *
 * Provides macros for rounding integer values and pointers down to a
 * specified alignment boundary.
 */

// https://github.com/akx/so55822235/blob/master/libc-pointer-arith.h

#ifndef NULLEX_POINTER_ARITH_H
#define NULLEX_POINTER_ARITH_H

/**
 * @brief Align an integer value down to the nearest alignment boundary.
 *
 * Rounds @p base down to the closest multiple of @p size.
 *
 * For example, with an alignment of 4096:
 *
 * @code
 * ALIGN_DOWN(4095, 4096) -> 0
 * ALIGN_DOWN(4096, 4096) -> 4096
 * ALIGN_DOWN(4097, 4096) -> 4096
 * @endcode
 *
 * @param[in] base Value to align.
 * @param[in] size Alignment boundary.
 * @return The value of @p base rounded down to the nearest multiple of
 *         @p size.
 *
 * @note @p size should normally be a power of two for this bitwise
 *       implementation to produce the expected alignment.
 */
#define ALIGN_DOWN(base, size) ((base) & -((__typeof__(base))(size)))

/**
 * @brief Align a pointer down to the nearest alignment boundary.
 *
 * Converts the pointer to an integer representation, aligns it down using
 * @c ALIGN_DOWN(), and converts the result back to the original pointer type.
 *
 * @param[in] base Pointer to align.
 * @param[in] size Alignment boundary.
 * @return The aligned pointer at or below @p base.
 *
 * @note @p size should normally be a power of two.
 */
#define PTR_ALIGN_DOWN(base, size) \
  ((__typeof__(base))ALIGN_DOWN((uintptr_t)(base), (size)))

#endif