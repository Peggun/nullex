/**
 * @file errno.h
 * @brief Error codes used by Nullex system and library interfaces.
 *
 * Defines negative integer error codes returned by Nullex functions and
 * system interfaces to indicate failure conditions.
 */

#ifndef NULLEX_ERRNO_H
#define NULLEX_ERRNO_H

/**
 * @brief Requested entry or resource could not be found.
 */
#define ERR_NO_ENT -1

/**
 * @brief The supplied file descriptor is invalid.
 */
#define ERR_BAD_FD -2

/**
 * @brief The requested operation requires a directory.
 */
#define ERR_NOT_DIR -3

/**
 * @brief The requested operation requires a file, but a directory was given.
 */
#define ERR_IS_DIR -4

/**
 * @brief No queue is available for the requested operation.
 */
#define ERR_NO_QUEUE -5

/**
 * @brief Failed to create a process.
 */
#define ERR_CRE_PROC -6

/**
 * @brief DNS resolution failed.
 */
#define ERR_DNS_FAIL -7

/**
 * @brief The supplied URL is invalid or malformed.
 */
#define ERR_BAD_URL -8

/**
 * @brief The requested network gateway could not be reached.
 */
#define ERR_GATEWAY_UNREACH -9

/**
 * @brief Failed to establish a TCP connection.
 */
#define ERR_TCP_CONN_FAIL -10

/**
 * @brief The requested operation requires an established connection,
 *        but no connection is currently available.
 */
#define ERR_NOT_CONNECTED -11

/**
 * @brief Failed to send the requested data.
 */
#define ERR_FAILED_TO_SEND -12

/**
 * @brief The requested operation exceeded its allowed time.
 */
#define ERR_TIMED_OUT -13

/**
 * @brief A TLS operation failed.
 */
#define ERR_TLS_FAILED -14

/**
 * @brief The supplied argument or operation is invalid.
 */
#define ERR_INVALID -15

#endif