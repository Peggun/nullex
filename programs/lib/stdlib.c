// https://www.youtube.com/watch?v=sZ8GJ1TiMdk&t=21s

#include <nullex/syscalls.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "heap.h"

static uint8_t heap[HEAP_CAPACITY]
    __attribute__((aligned(HEAP_ALIGNMENT))) = {0};

static size_t heap_size = 0;

static void validate_heap_size(const char *where) {
  if (heap_size > HEAP_CAPACITY) {
    printf("[HEAP CORRUPT] %s: heap_size=%zu (0x%zx), addr=%p\n", where,
           heap_size, heap_size, &heap_size);

    abort();
  }
}

static HeapChunkHeader *heap_head = NULL;
static HeapChunkHeader *heap_tail = NULL;

static bool align_size(size_t size, size_t *out) {
  if (size > (size_t)-1 - (HEAP_ALIGNMENT - 1)) return false;

  *out = (size + HEAP_ALIGNMENT - 1) & ~((size_t)HEAP_ALIGNMENT - 1);

  return true;
}

static void *chunk_data(HeapChunkHeader *chunk) {
  return (uint8_t *)chunk + HEAP_HEADER_SIZE;
}

static bool chunks_are_adjacent(HeapChunkHeader *left, HeapChunkHeader *right) {
  uint8_t *left_end = (uint8_t *)left + HEAP_HEADER_SIZE + left->size;

  return left_end == (uint8_t *)right;
}

static HeapChunkHeader *find_chunk(void *ptr) {
  HeapChunkHeader *chunk = heap_head;

  while (chunk != NULL) {
    if (chunk_data(chunk) == ptr) return chunk;

    chunk = chunk->next;
  }

  return NULL;
}

static HeapChunkHeader *find_free_chunk(size_t size) {
  HeapChunkHeader *chunk = heap_head;

  while (chunk != NULL) {
    if (chunk->free && chunk->size >= size) return chunk;

    chunk = chunk->next;
  }

  return NULL;
}

static void split_chunk(HeapChunkHeader *chunk, size_t size) {
  if (chunk->size < size) return;

  size_t remaining = chunk->size - size;

  if (remaining < HEAP_HEADER_SIZE + HEAP_ALIGNMENT) return;

  HeapChunkHeader *new_chunk =
      (HeapChunkHeader *)((uint8_t *)chunk + HEAP_HEADER_SIZE + size);

  new_chunk->size = remaining - HEAP_HEADER_SIZE;

  new_chunk->prev = chunk;
  new_chunk->next = chunk->next;
  new_chunk->free = true;

  if (new_chunk->next != NULL)
    new_chunk->next->prev = new_chunk;
  else
    heap_tail = new_chunk;

  chunk->next = new_chunk;
  chunk->size = size;
}

static HeapChunkHeader *merge_with_next(HeapChunkHeader *chunk) {
  HeapChunkHeader *next = chunk->next;

  if (next == NULL) return chunk;

  if (!chunk->free || !next->free) return chunk;

  if (!chunks_are_adjacent(chunk, next)) return chunk;

  chunk->size += HEAP_HEADER_SIZE + next->size;

  chunk->next = next->next;

  if (chunk->next != NULL)
    chunk->next->prev = chunk;
  else
    heap_tail = chunk;

  return chunk;
}

static HeapChunkHeader *coalesce_chunk(HeapChunkHeader *chunk) {
  if (chunk == NULL || !chunk->free) return chunk;

  /*
   * Merge backwards first.
   */
  if (chunk->prev != NULL && chunk->prev->free &&
      chunks_are_adjacent(chunk->prev, chunk)) {
    chunk = merge_with_next(chunk->prev);
  }

  /*
   * Then merge forwards until there is nothing left to merge.
   */
  while (chunk->next != NULL && chunk->next->free &&
         chunks_are_adjacent(chunk, chunk->next)) {
    chunk = merge_with_next(chunk);
  }

  return chunk;
}

static void release_top_free_chunks(void) {
  while (heap_tail != NULL && heap_tail->free) {
    HeapChunkHeader *chunk = heap_tail;

    uint8_t *chunk_end = (uint8_t *)chunk + HEAP_HEADER_SIZE + chunk->size;

    if (chunk_end != heap + heap_size) return;

    size_t released_size = HEAP_HEADER_SIZE + chunk->size;

    heap_tail = chunk->prev;

    if (heap_tail != NULL)
      heap_tail->next = NULL;
    else
      heap_head = NULL;

    heap_size -= released_size;
  }
}

void *malloc(size_t size) {
  validate_heap_size("malloc entry");

  if (size == 0) return NULL;

  size_t aligned_size;
  if (!align_size(size, &aligned_size)) return NULL;

  HeapChunkHeader *chunk = find_free_chunk(aligned_size);

  validate_heap_size("after find_free_chunk");

  if (chunk != NULL) {
    split_chunk(chunk, aligned_size);

    validate_heap_size("after split_chunk");

    chunk->free = false;

    validate_heap_size("after chunk free=false");

    return chunk_data(chunk);
  }

  size_t required_size = HEAP_HEADER_SIZE + aligned_size;

  if (required_size > HEAP_CAPACITY - heap_size) return NULL;

  chunk = (HeapChunkHeader *)(heap + heap_size);

  validate_heap_size("before header write");

  chunk->size = aligned_size;
  validate_heap_size("after chunk->size");

  chunk->prev = heap_tail;
  validate_heap_size("after chunk->prev");

  chunk->next = NULL;
  validate_heap_size("after chunk->next");

  chunk->free = false;
  validate_heap_size("after chunk->free");

  if (heap_tail != NULL) {
    heap_tail->next = chunk;
    validate_heap_size("after heap_tail->next");
  } else {
    heap_head = chunk;
  }

  heap_tail = chunk;

  validate_heap_size("before heap_size increment");

  heap_size += required_size;

  validate_heap_size("after heap_size increment");

  return chunk_data(chunk);
}

void *calloc(size_t nmemb, size_t size) {
  if (size != 0 && nmemb > (size_t)-1 / size) {
    return NULL;
  }

  size_t total = nmemb * size;

  uint8_t *result = malloc(total);

  if (result == NULL) return NULL;

  for (size_t i = 0; i < total; ++i) result[i] = 0;

  return result;
}

void *alloc(size_t size) { return malloc(size); }

void free(void *ptr) {
  if (ptr == NULL) return;

  /*
   * Find the chunk rather than blindly trusting ptr - HEADER_SIZE.
   * This means invalid pointers and double frees don't immediately
   * corrupt the allocator metadata.
   */
  HeapChunkHeader *chunk = find_chunk(ptr);

  if (chunk == NULL) return;

  if (chunk->free) return;

  chunk->free = true;

  coalesce_chunk(chunk);
  release_top_free_chunks();
}

void collect(void) {
  HeapChunkHeader *chunk = heap_head;

  while (chunk != NULL) {
    if (chunk->free) {
      chunk = coalesce_chunk(chunk);
    }

    chunk = chunk->next;
  }

  release_top_free_chunks();
}

void abort(void) { halt(134); }

void heap_dump_alloced_chunks(void) {
  size_t count = 0;

  HeapChunkHeader *chunk = heap_head;

  while (chunk != NULL) {
    if (!chunk->free) ++count;

    chunk = chunk->next;
  }

  printf("Allocated chunks (%zu), heap usage %zu/%zu:\n", count, heap_size,
         (size_t)HEAP_CAPACITY);

  chunk = heap_head;

  while (chunk != NULL) {
    if (!chunk->free) {
      printf("    start: %p, size: %zu\n", chunk_data(chunk), chunk->size);
    }

    chunk = chunk->next;
  }
}