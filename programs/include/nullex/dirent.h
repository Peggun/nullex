#ifndef NULLEX_DIRENT_H
#define NULLEX_DIRENT_H

#include "../stdbool.h"
#include "../stddef.h"
#include "../stdint.h"

#define NULLEX_NAME_MAX 256

#define OPEND_NONE    ((uint64_t)0)
#define OPEND_RESOLVE ((uint64_t)1)

typedef struct Permission {
    uint8_t read;
    uint8_t write;
    uint8_t execute;
} Permission;

typedef enum FsError {
    FS_ERR_ENTRY_NOT_FOUND = 1,
    FS_ERR_NOT_A_DIRECTORY = 2,
    FS_ERR_NOT_A_FILE = 3,
    FS_ERR_PERMISSION_DENIED = 4,
    FS_ERR_ALREADY_EXISTS = 5,
    FS_ERR_INVALID_PATH = 6,
    FS_ERR_DIRECTORY_NOT_EMPTY = 7,
} FsError;

typedef enum EntryKind {
    ENTRY_FILE = 0,
    ENTRY_DIRECTORY = 1,
} EntryKind;

typedef struct Entry Entry;
typedef struct File File;
typedef struct Directory Directory;
typedef struct ChunkedContent ChunkedContent;
typedef struct DirEntryInfo DirEntryInfo;

struct Directory {
    Entry *entries;
    size_t entry_count;
    size_t entry_capacity;
    Permission permission;
};

struct ChunkedContent {
    uint8_t **chunks;
    size_t chunk_count;
    size_t length;
};

struct File {
    uint8_t *content;
    size_t len;
    size_t capacity;
    Permission permission;
};

struct Entry {
    EntryKind kind;
    union {
        File *file;
        Directory *directory;
    } as;
};

struct DirEntryInfo {
    EntryKind kind;
    Permission permission;
    uint64_t size;
    uint32_t name_len;
    char name[NULLEX_NAME_MAX];
};

#endif