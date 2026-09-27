/**
 * @file stdarg.h
 * @brief Support for handling variable-length function arguments.
 *
 * Provides the types and macros required to access arguments passed to
 * functions with a variable number of parameters.
 */

#ifndef NULLEX_STDARG_H
#define NULLEX_STDARG_H

/**
 * @brief Represents the state of a variable argument list.
 *
 * This type is provided by the compiler and is used internally by the
 * variable argument handling macros.
 */
typedef __builtin_va_list va_list;

/**
 * @brief Initialise a variable argument list.
 *
 * Initialises @p ap so that it refers to the first unnamed argument passed
 * to the current function.
 *
 * @param[out] ap Variable argument list to initialise.
 * @param[in] param The last named parameter of the current function.
 */
#define va_start(ap, param) __builtin_va_start(ap, param)

/**
 * @brief Retrieve the next argument from a variable argument list.
 *
 * Retrieves the next argument from @p ap and advances the argument list to
 * the following argument.
 *
 * @param[in,out] ap Variable argument list to read from.
 * @param[in] type Type of the argument to retrieve.
 * @return The next argument of the specified @p type.
 */
#define va_arg(ap, type) __builtin_va_arg(ap, type)

/**
 * @brief End access to a variable argument list.
 *
 * Releases any resources associated with @p ap and invalidates the argument
 * list for further use.
 *
 * @param[in,out] ap Variable argument list to terminate.
 */
#define va_end(ap) __builtin_va_end(ap)

/**
 * @brief Create a copy of a variable argument list.
 *
 * Initialises @p dest as a copy of the current state of @p src. The two
 * argument lists can subsequently be traversed independently.
 *
 * @param[out] dest Destination variable argument list.
 * @param[in] src Source variable argument list to copy.
 */
#define va_copy(dest, src) __builtin_va_copy(dest, src)

#endif