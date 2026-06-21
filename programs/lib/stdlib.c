// https://www.youtube.com/watch?v=sZ8GJ1TiMdk&t=21s

#include "../include/stdio.h"
#include "../include/stdlib.h"
#include "../include/stdbool.h"
#include "../include/stdint.h"

#define HEAP_ALIGNMENT 16

static uint8_t heap[HEAP_CAPACITY] __attribute__((aligned(HEAP_ALIGNMENT))) = {0};
static size_t heap_size = 0;

static HeapChunk heap_alloced[HEAP_ALLOCED_CAP] = {0};
static size_t heap_alloced_size = 0;

static HeapChunk heap_free[HEAP_ALLOCED_CAP] = {0};
static size_t heap_free_size = 0;

static bool align_size(size_t size, size_t *out)
{
    if (size > (size_t)-1 - (HEAP_ALIGNMENT - 1))
        return false;

    *out = (size + HEAP_ALIGNMENT - 1) & ~((size_t)HEAP_ALIGNMENT - 1);
    return true;
}

static void remove_chunk(HeapChunk *chunks, size_t *chunk_count, size_t index)
{
    if (index >= *chunk_count)
        return;

    for (size_t i = index; i + 1 < *chunk_count; ++i) {
        chunks[i] = chunks[i + 1];
    }

    *chunk_count -= 1;
    chunks[*chunk_count] = (HeapChunk){0};
}

static bool record_alloced_chunk(void *ptr, size_t size)
{
    if (heap_alloced_size >= HEAP_ALLOCED_CAP)
        return false;

    heap_alloced[heap_alloced_size++] = (HeapChunk){
        .start = ptr,
        .size = size
    };

    return true;
}

static bool find_alloced_chunk(void *ptr, size_t *index)
{
    for (size_t i = 0; i < heap_alloced_size; ++i) {
        if (heap_alloced[i].start == ptr) {
            *index = i;
            return true;
        }
    }

    return false;
}

static bool take_free_chunk(size_t size, void **out)
{
    for (size_t i = 0; i < heap_free_size; ++i) {
        if (heap_free[i].size < size)
            continue;

        uint8_t *start = (uint8_t *)heap_free[i].start;
        *out = start;

        if (heap_free[i].size == size) {
            remove_chunk(heap_free, &heap_free_size, i);
        } else {
            heap_free[i].start = start + size;
            heap_free[i].size -= size;
        }

        return true;
    }

    return false;
}

static void coalesce_free_chunks(void)
{
    for (size_t i = 0; i + 1 < heap_free_size;) {
        uint8_t *end = (uint8_t *)heap_free[i].start + heap_free[i].size;

        if (end == (uint8_t *)heap_free[i + 1].start) {
            heap_free[i].size += heap_free[i + 1].size;
            remove_chunk(heap_free, &heap_free_size, i + 1);
        } else {
            ++i;
        }
    }
}

static void release_top_free_chunks(void)
{
    while (heap_free_size > 0) {
        size_t last = heap_free_size - 1;
        uint8_t *end = (uint8_t *)heap_free[last].start + heap_free[last].size;

        if (end != heap + heap_size)
            return;

        heap_size -= heap_free[last].size;
        remove_chunk(heap_free, &heap_free_size, last);
    }
}

static void insert_free_chunk(HeapChunk chunk)
{
    if (heap_free_size >= HEAP_ALLOCED_CAP)
        return;

    size_t pos = 0;
    uintptr_t chunk_start = (uintptr_t)chunk.start;
    while (pos < heap_free_size && (uintptr_t)heap_free[pos].start < chunk_start) {
        ++pos;
    }

    for (size_t i = heap_free_size; i > pos; --i) {
        heap_free[i] = heap_free[i - 1];
    }

    heap_free[pos] = chunk;
    heap_free_size += 1;

    coalesce_free_chunks();
    release_top_free_chunks();
}

void *malloc(size_t size)
{
    if (size == 0)
        return NULL;

    if (heap_alloced_size >= HEAP_ALLOCED_CAP)
        return NULL;

    size_t aligned_size;
    if (!align_size(size, &aligned_size))
        return NULL;

    void *result = NULL;
    if (take_free_chunk(aligned_size, &result)) {
        record_alloced_chunk(result, aligned_size);
        return result;
    }

    if (heap_size > HEAP_CAPACITY || aligned_size > HEAP_CAPACITY - heap_size)
        return NULL;

    result = heap + heap_size;
    heap_size += aligned_size;

    if (!record_alloced_chunk(result, aligned_size))
        return NULL;

    return result;
}

void *calloc(size_t nmemb, size_t size)
{
    if (size != 0 && nmemb > (size_t)-1 / size)
        return NULL;

    size_t total = nmemb * size;
    uint8_t *result = malloc(total);
    if (result == NULL)
        return NULL;

    for (size_t i = 0; i < total; ++i) {
        result[i] = 0;
    }

    return result;
}

void *alloc(size_t size)
{
    return malloc(size);
}

void free(void *ptr) 
{
    if (ptr == NULL)
        return;

    size_t index = 0;
    if (!find_alloced_chunk(ptr, &index))
        return;

    HeapChunk chunk = heap_alloced[index];
    remove_chunk(heap_alloced, &heap_alloced_size, index);
    insert_free_chunk(chunk);
}

void collect()
{
    coalesce_free_chunks();
    release_top_free_chunks();
}

void heap_dump_alloced_chunks(void) 
{
    size_t count = heap_alloced_size;
    if (count > HEAP_ALLOCED_CAP) {
        printf("Allocated chunks metadata corrupt (%zu), capping dump at %u:\n",
               heap_alloced_size,
               HEAP_ALLOCED_CAP
        );
        count = HEAP_ALLOCED_CAP;
    } else {
        printf("Allocated chunks (%zu):\n", count);
    }

    for (size_t i = 0; i < count; ++i) {
        printf("    start: %p, size: %zu\n",
               heap_alloced[i].start,
               heap_alloced[i].size
        );
    }
}
