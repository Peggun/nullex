#ifndef _STDLIB_H
#define _STDLIB_H

#include "stddef.h"

#define HEAP_CAPACITY 640000
#define HEAP_ALLOCED_CAP 1024

typedef struct {
    void *start;
    size_t size;
} HeapChunk;

void *malloc(size_t size);
void *calloc(size_t nmemb, size_t size);
void *alloc(size_t size);
void free(void *ptr);

// garbage collection
void collect();

void heap_dump_alloced_chunks(void);

#endif
