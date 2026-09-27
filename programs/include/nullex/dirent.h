#ifndef NULLEX_DIRENT_H
#define NULLEX_DIRENT_H

#include <nullex/syscalls.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>

/**
 * @brief Maximum length of a filesystem entry name.
 *
 * This value includes space for the terminating null character.
 */
#define NULLEX_NAME_MAX 256

/**
 * @brief Open-directory operation that performs no special operation.
 */
#define OPEND_NONE ((uint64_t)0)

/**
 * @brief Open-directory operation that resolves the requested path.
 */
#define OPEND_RESOLVE ((uint64_t)1)

/**
 * @brief Defines the permissions associated with a filesystem object.
 */
typedef struct Permission {
  /** @brief Whether the object may be read. */
  uint8_t read;

  /** @brief Whether the object may be written to. */
  uint8_t write;

  /** @brief Whether the object may be executed. */
  uint8_t execute;
} Permission;

/**
 * @brief Identifies the type of a filesystem entry.
 */
typedef enum EntryKind {
  /** @brief The entry represents a regular file. */
  ENTRY_FILE = 0,

  /** @brief The entry represents a directory. */
  ENTRY_DIRECTORY = 1,
} EntryKind;

/** @brief Forward declaration of a filesystem entry. */
typedef struct Entry Entry;

/** @brief Forward declaration of a file. */
typedef struct File File;

/** @brief Forward declaration of a directory. */
typedef struct Directory Directory;

/** @brief Forward declaration of chunked file content. */
typedef struct ChunkedContent ChunkedContent;

/** @brief Forward declaration of directory entry metadata. */
typedef struct DirEntryInfo DirEntryInfo;

/**
 * @brief Represents a directory in the filesystem.
 */
struct Directory {
  /** @brief Array containing the entries stored in the directory. */
  Entry *entries;

  /** @brief Number of entries currently stored in the directory. */
  size_t entry_count;

  /** @brief Allocated capacity of the @c entries array. */
  size_t entry_capacity;

  /** @brief Permissions associated with the directory. */
  Permission permission;
};

/**
 * @brief Represents file content stored in dynamically sized chunks.
 *
 * Chunked storage allows file contents to be represented without requiring
 * one contiguous memory allocation.
 */
struct ChunkedContent {
  /** @brief Array of pointers to individual content chunks. */
  uint8_t **chunks;

  /** @brief Number of chunks currently allocated. */
  size_t chunk_count;

  /** @brief Total length of the content in bytes. */
  size_t length;
};

/**
 * @brief Represents a regular file in the filesystem.
 */
struct File {
  /** @brief Pointer to the file's content. */
  uint8_t *content;

  /** @brief Current length of the file content in bytes. */
  size_t len;

  /** @brief Allocated capacity of the file content buffer in bytes. */
  size_t capacity;

  /** @brief Permissions associated with the file. */
  Permission permission;
};

/**
 * @brief Represents a filesystem entry.
 *
 * An entry can represent either a regular file or a directory. The active
 * member of @c as is determined by @c kind.
 */
struct Entry {
  /** @brief Type of the filesystem entry. */
  EntryKind kind;

  /**
   * @brief Underlying object represented by this entry.
   *
   * The @c file member is valid when @c kind is @c ENTRY_FILE, while the
   * @c directory member is valid when @c kind is @c ENTRY_DIRECTORY.
   */
  union {
    /** @brief Pointer to the underlying file. */
    File *file;

    /** @brief Pointer to the underlying directory. */
    Directory *directory;
  } as;
};

/**
 * @brief Describes metadata for a directory entry.
 *
 * This structure is intended to provide information about an entry without
 * exposing the complete underlying @c File or @c Directory structure.
 */
struct DirEntryInfo {
  /** @brief Type of the filesystem entry. */
  EntryKind kind;

  /** @brief Permissions associated with the entry. */
  Permission permission;

  /** @brief Size of the entry in bytes. */
  uint64_t size;

  /** @brief Length of the entry name, excluding the terminating null byte. */
  uint32_t name_len;

  /**
   * @brief Null-terminated name of the filesystem entry.
   *
   * The maximum length of the stored name is @c NULLEX_NAME_MAX.
   */
  char name[NULLEX_NAME_MAX];
};

/**
 * @brief Open a filesystem path directory.
 *
 * @param[in] path Null-terminated filesystem path.
 * @param[in] flags Open operation flags.
 * @return File or object descriptor on success, otherwise a negative error
 *         code.
 */
static inline int32_t opend(const char *path, int32_t flags) {
  size_t len = strlen(path);

  return ksyscall(SYS_OPEND, (uint64_t)path, (uint64_t)len, (uint64_t)flags, 0,
                  0, 0);
}

/**
 * @brief Close a directory descriptor.
 *
 * @param[in] fd Directory descriptor.
 * @return Result returned by the @c SYS_CLOSED syscall.
 */
static inline int32_t closed(int fd) {
  return ksyscall(SYS_CLOSED, (uint64_t)fd, 0, 0, 0, 0, 0);
}

/**
 * @brief Retrieve directory entries from a directory descriptor.
 *
 * @param[in] fd Directory descriptor.
 * @param[out] dei Buffer receiving directory entry information.
 * @param[in] out_cap Capacity of the output buffer.
 * @return Number of entries or bytes returned, depending on the syscall
 *         contract, or a negative error code.
 */
static inline int32_t getdirents(uint64_t fd, DirEntryInfo *dei,
                                 uintptr_t out_cap) {
  return ksyscall(SYS_GETDIRENTS, fd, (uint64_t)dei, (uint64_t)out_cap, 0, 0,
                  0);
}

#endif