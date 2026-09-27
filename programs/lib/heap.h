#include <stdbool.h>
#include <stddef.h>

#define HEAP_CAPACITY 640000
#define HEAP_ALLOCED_CAP 1024
#define HEAP_ALIGNMENT 32

typedef struct HeapChunkHeader {
  size_t size;

  struct HeapChunkHeader *prev;
  struct HeapChunkHeader *next;

  bool free;
} HeapChunkHeader;

#define HEAP_HEADER_SIZE ((sizeof(HeapChunkHeader) + HEAP_ALIGNMENT - 1) & ~(HEAP_ALIGNMENT - 1))