#ifndef NULLEX_STDLIB_H
#define NULLEX_STDLIB_H

#include <stddef.h>

/**
 * @brief Allocate a block of memory.
 *
 * @param[in] size Number of bytes to allocate.
 * @return Pointer to the allocated memory, or @c NULL on failure.
 */
void *malloc(size_t size);

/**
 * @brief Allocate and zero-initialise an array of objects.
 *
 * @param[in] nmemb Number of objects to allocate.
 * @param[in] size Size of each object in bytes.
 * @return Pointer to the allocated memory, or @c NULL on failure.
 */
void *calloc(size_t nmemb, size_t size);

/**
 * @brief Allocate a block of memory using the Nullex allocator.
 *
 * This is an alias or lower-level form of @c malloc() if exposed as part
 * of the Nullex allocation API.
 *
 * @param[in] size Number of bytes to allocate.
 * @return Pointer to the allocated memory, or @c NULL on failure.
 */
void *alloc(size_t size);

/**
 * @brief Release previously allocated memory.
 *
 * @param[in] ptr Pointer returned by an allocation function.
 */
void free(void *ptr);

/**
 * @brief Immediately terminate the current process.
 */
void abort(void);

#endif