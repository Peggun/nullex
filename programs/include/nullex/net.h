/**
 * @file net.h
 * @brief Networking types and message I/O structures.
 *
 * Provides the common types used by the Nullex networking interface,
 * including socket identifiers, scatter/gather I/O vectors, and message
 * descriptors.
 */

#ifndef NULLEX_NET_H
#define NULLEX_NET_H

#include <nullex/syscalls.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

/**
 * @brief Identifies a network socket.
 *
 * A socket descriptor is represented as an integer handle used to refer
 * to a socket through the Nullex networking interface.
 */
typedef int socket_t;

/**
 * @brief Specifies the type of I/O payload contained in a message.
 */
typedef enum {
  /** @brief Message contains a single contiguous memory buffer. */
  NLX_IO_RAW = 0,

  /** @brief Message contains multiple memory buffers. */
  NLX_IO_VEC = 1
} nlx_io_type_t;

/**
 * @brief Describes a single memory buffer used for I/O.
 *
 * An I/O vector describes a contiguous region of memory that may be used
 * as part of a scatter/gather I/O operation.
 */
struct nlx_iovec {
  /**
   * @brief Pointer to the start of the memory buffer.
   */
  const void *iov_base;

  /**
   * @brief Length of the memory buffer in bytes.
   */
  size_t iov_len;
};

/**
 * @brief Describes the payload of a network message.
 *
 * The payload may either consist of one contiguous buffer or a collection
 * of I/O vectors, as specified by @c type.
 */
struct nlx_msghdr {
  /**
   * @brief Type of I/O payload stored in @c payload.
   */
  nlx_io_type_t type;

  /**
   * @brief Reserved padding for structure layout and alignment.
   *
   * This field is not intended to be accessed or modified by callers.
   */
  uint32_t _padding;

  /**
   * @brief Message payload.
   *
   * The active member is determined by @c type.
   */
  union {
    /**
     * @brief Contiguous raw payload.
     */
    struct {
      /**
       * @brief Pointer to the payload buffer.
       */
      const void *buf;

      /**
       * @brief Length of the payload in bytes.
       */
      size_t len;
    } raw;

    /**
     * @brief Scatter/gather vector payload.
     */
    struct {
      /**
       * @brief Array of I/O vectors making up the message.
       */
      const struct nlx_iovec *msg_iov;

      /**
       * @brief Number of I/O vectors in @c msg_iov.
       */
      size_t msg_iovcnt;
    } vec;
  } payload;
};

/**
 * @brief Create a network socket.
 *
 * @return Socket descriptor on success, otherwise a negative error code.
 */
static inline int32_t csocket(void) {
  return ksyscall(SYS_CSOCKET, 0, 0, 0, 0, 0, 0);
}

/**
 * @brief Connect a socket to a remote host.
 *
 * @param[in] fd Socket descriptor.
 * @param[in] host Null-terminated hostname.
 * @param[in] port Remote TCP port.
 * @return Result returned by the @c SYS_CONNSOCK syscall.
 */
static inline int32_t connsock(int fd, const char *host, int port) {
  size_t len = strlen(host);

  return ksyscall(SYS_CONNSOCK, (uint64_t)fd, (uint64_t)host, (uint64_t)len,
                  (uint64_t)port, 0, 0);
}

/**
 * @brief Send data through a socket.
 *
 * @param[in] fd Socket descriptor.
 * @param[in] data Buffer containing the data to send.
 * @param[in] len Number of bytes to send.
 * @return Number of bytes sent or a negative error code.
 */
static inline int32_t send(int fd, const uint8_t *data, size_t len) {
  return ksyscall(SYS_SEND, (uint64_t)fd, (uint64_t)data, (uint64_t)len, 0, 0,
                  0);
}

/**
 * @brief Receive data from a socket.
 *
 * @param[in] fd Socket descriptor.
 * @param[out] msg Message descriptor receiving the network data.
 * @return Number of bytes received or a negative error code.
 */
static inline int32_t recv(int fd, struct nlx_msghdr *msg) {
  return ksyscall(SYS_RECV, (uint64_t)fd, (uint64_t)msg, 0, 0, 0, 0);
}

/**
 * @brief Close a socket.
 *
 * @param[in] fd Socket descriptor.
 * @return 0 on success, otherwise a negative error code.
 */
static inline int32_t closesock(int fd) {
  return ksyscall(SYS_CLOSESOCK, (uint64_t)fd, 0, 0, 0, 0, 0);
}

#endif