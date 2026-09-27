/*
    @file fs.h
    @brief Filesystem and path manipulation utilities for the Nullex kernel

    This header provides utilities for normalizing, resolving, and joining
   paths, as well as computing the SHA-256 hash of a file.x


 */

#ifndef NULLEX_FS_H
#define NULLEX_FS_H

#include <nullex/syscalls.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

/** @brief Maximum length supported for a file path string, including the null
 * terminator. */
#define MAX_PATH_LEN 1024

/** @brief Maximum number of path parts supported for path joining. */
#define MAX_PARTS 128

/** @name File Access Flags */
/**@{*/
#define O_RDONLY 0x1 /**< Read-only access. */
#define O_WRONLY 0x2 /**< Write-only access. */
#define O_RDWR 0x4   /**< Read/write access. */
#define O_CREAT 0x8  /**< Create the file if it does not exist. */
/**@}*/

/**
 * @brief Errors that may be returned by filesystem operations.
 */
typedef enum FsError {
  /** @brief The requested filesystem entry could not be found. */
  FS_ERR_ENTRY_NOT_FOUND = 1,

  /** @brief The specified entry is not a directory. */
  FS_ERR_NOT_A_DIRECTORY = 2,

  /** @brief The specified entry is not a file. */
  FS_ERR_NOT_A_FILE = 3,

  /** @brief The requested operation is not permitted. */
  FS_ERR_PERMISSION_DENIED = 4,

  /** @brief An entry with the requested name already exists. */
  FS_ERR_ALREADY_EXISTS = 5,

  /** @brief The specified path is invalid. */
  FS_ERR_INVALID_PATH = 6,

  /** @brief The directory contains entries and cannot be removed. */
  FS_ERR_DIRECTORY_NOT_EMPTY = 7,
} FsError;

/**
 * @brief Normalises a path string by resolving relative elements like '.' and
 * '..'.
 *
 * Eliminates redundant slashes and processes directory traversal tokens to
 * yield an absolute, canonicalised path string.
 *
 * @param[in]  path     The raw input path string to normalise.
 * @param[out] out      Pointer to the buffer where the normalised path will be
 * written.
 * @param[in]  out_size Size of the output buffer in bytes.
 *
 * @return Pointer to @p out on success, or NULL if an error occurs (e.g., null
 * pointers, insufficient buffer size, or exceeding #MAX_PARTS).
 */
char *normpath(const char *path, char *out, size_t out_size);

/**
 * @brief Resolves a relative or absolute path against a current working
 * directory.
 *
 * If the input path is relative, it joins it with the provided current working
 * directory before normalising the final result. If the path is absolute, it is
 * normalised directly.
 *
 * @param[in]  path     The relative or absolute target path.
 * @param[in]  cwd      The current working directory context.
 * @param[out] out      Pointer to the buffer to receive the fully resolved
 * path.
 * @param[in]  out_size Size of the destination buffer in bytes.
 *
 * @return Pointer to @p out on success, or NULL on failure.
 */
char *rslvpath(const char *path, const char *cwd, char *out, size_t out_size);

/**
 * @brief Formats and concatenates a working directory context with a relative
 * path.
 *
 * Directly joins two paths with a forward slash separator. This function does
 * not evaluate intermediate '.' or '..' segments.
 *
 * @param[in]  cwd      The base directory path string.
 * @param[in]  path     The path string to append.
 * @param[out] out      Pointer to the buffer where the combined path will be
 * written.
 * @param[in]  out_size Size of the destination buffer in bytes.
 *
 * @return Pointer to @p out on success, or NULL if the buffer overflows.
 */
char *joinpath(const char *cwd, const char *path, char *out, size_t out_size);

/**
 * @brief Computes the SHA-256 cryptographic checksum of a specified file.
 *
 * Reads the target file, calculates its SHA-256 hash stream, and outputs the
 * result as a null-terminated lowercase hexadecimal string.
 *
 * @param[in]  filename   Path to the file to be hashed.
 * @param[out] output_hex Buffer to store the resulting hex string. Must be at
 * least 65 bytes long (64 characters + null terminator).
 *
 * @return 0 on success, or -1 if the file cannot be opened or a read error
 * occurs.
 */
int compute_sha256_file(const char *filename, char *output_hex);

/**
 * @brief Opens a file with the specified path and flags.
 *
 * @param[in] path  Path to the file to open.
 * @param[in] flags Flags for opening the file (e.g., O_RDONLY, O_WRONLY,
 * O_CREAT).
 *
 * @return File descriptor on success, or a negative value if the file cannot be
 * opened.
 */
static inline int32_t openf(const char *path, uint32_t flags) {
  size_t len = strlen(path);
  return ksyscall(SYS_OPENF, (uint64_t)path, (uint64_t)len, (uint64_t)flags, 0,
                  0, 0);
}

/**
 * @brief Closes the file descriptor.
 *
 * @param[in] fd File descriptor to close.
 *
 * @return 0 on success, or a negative value if the file descriptor cannot be
 * closed.
 */
static inline int32_t closef(uint64_t fd) {
  return ksyscall(SYS_CLOSEF, fd, 0, 0, 0, 0, 0);
}

/**
 * @brief Reads data from the file descriptor.
 *
 * @param[in] fd  File descriptor to read from.
 * @param[out] buf Buffer to store the read data.
 * @param[in] len  Number of bytes to read.
 *
 * @return Number of bytes read on success, or a negative value if the read
 * operation fails.
 */
static inline int32_t readf(uint64_t fd, uint8_t *buf, size_t len) {
  return ksyscall(SYS_READF, fd, (uint64_t)buf, (uint64_t)len, 0, 0, 0);
}

/**
 * @brief Writes data in a buffer to the file descriptor.
 *
 * @param[in] fd     File descriptor to write to.
 * @param[in] buf    Buffer containing the data to write.
 * @param[in] len    Number of bytes to write.
 *
 * @return Number of bytes written on success, or a negative value if the write
 * operation fails.
 */
static inline int32_t writef_buf(uint64_t fd, uint8_t *buf, size_t len) {
  return ksyscall(SYS_WRITEF, fd, (uint64_t)buf, (uint64_t)len, 0, 0, 0);
}

/**
 * @brief Writes a string to the file descriptor.
 *
 * @param[in] fd        File descriptor to write to.
 * @param[in] to_write  String to write.
 *
 * @return Number of bytes written on success, or a negative value if the write
 * operation fails.
 */
static inline int32_t writef_str(uint64_t fd, const char *to_write) {
  size_t len = strlen(to_write);
  return ksyscall(SYS_WRITEF, fd, (uint64_t)to_write, (uint64_t)len, 0, 0, 0);
}

/**
 * @brief Writes data to the file descriptor using a generic macro.
 *
 * @param[in] fd  File descriptor to write to.
 * @param[in] arg Data to write.
 * @param[in] ... Additional arguments if the data is a string or buffer.
 *
 * @return Number of bytes written on success, or a negative value if the write
 * operation fails.
 */
#define writef(fd, arg, ...)    \
  _Generic((arg),               \
      const char *: writef_str, \
      char *: writef_str,       \
      uint8_t *: writef_buf,    \
      const uint8_t *: writef_buf)(fd, arg, ##__VA_ARGS__)

/**
 * @brief Obtain the size of a file.
 *
 * @param[in] fd File descriptor.
 * @return File size on success, otherwise a negative error code.
 */
static inline int32_t sizef(uint64_t fd) {
  return ksyscall(SYS_SIZEF, fd, 0, 0, 0, 0, 0);
}

/**
 * @brief Remove a file.
 *
 * @param[in] path Null-terminated path to the file to remove.
 * @return Result returned by the @c SYS_RMFILE syscall.
 */
static inline int32_t rmfile(const char *path) {
  size_t len = strlen(path);

  return ksyscall(SYS_RMFILE, (uint64_t)path, (uint64_t)len, 0, 0, 0, 0);
}

/**
 * @brief Remove a directory.
 *
 * @param[in] path Null-terminated path to the directory to remove.
 * @return Result returned by the @c SYS_RMDIR syscall.
 */
static inline int32_t rmdir(const char *path) {
  size_t len = strlen(path);

  return ksyscall(SYS_RMDIR, (uint64_t)path, (uint64_t)len, 0, 0, 0, 0);
}

#endif