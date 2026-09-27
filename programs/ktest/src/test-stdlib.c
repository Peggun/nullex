#include "ktest.h"
#include "assert.h"

#include <stdlib.h>
#include <string.h>

int test_malloc_returns_usable_memory(void) {
  unsigned char *p = malloc(64);
  T_ASSERT_NOT_NULL(p);

  for (int i = 0; i < 64; i++) {
    p[i] = (unsigned char)i;
  }
  for (int i = 0; i < 64; i++) {
    T_ASSERT_EQ(p[i], (unsigned char)i);
  }

  free(p);
  return 0;
}
CREATE_TEST(test_malloc_returns_usable_memory)

int test_malloc_distinct_allocations_do_not_overlap(void) {
  char *a = malloc(32);
  char *b = malloc(32);

  T_ASSERT_NOT_NULL(a);
  T_ASSERT_NOT_NULL(b);
  T_ASSERT(a != b);

  memset(a, 0xAA, 32);
  memset(b, 0xBB, 32);

  for (int i = 0; i < 32; i++) {
    T_ASSERT_EQ((unsigned char)a[i], 0xAA);
    T_ASSERT_EQ((unsigned char)b[i], 0xBB);
  }

  free(a);
  free(b);
  return 0;
}
CREATE_TEST(test_malloc_distinct_allocations_do_not_overlap)

int test_calloc_zero_initializes(void) {
  int *arr = calloc(10, sizeof(int));
  T_ASSERT_NOT_NULL(arr);

  for (int i = 0; i < 10; i++) {
    T_ASSERT_EQ(arr[i], 0);
  }

  free(arr);
  return 0;
}
CREATE_TEST(test_calloc_zero_initializes)

int test_alloc_returns_usable_memory(void) {
  unsigned char *p = alloc(16);
  T_ASSERT_NOT_NULL(p);

  memset(p, 0x5A, 16);
  for (int i = 0; i < 16; i++) {
    T_ASSERT_EQ(p[i], (unsigned char)0x5A);
  }

  free(p);
  return 0;
}
CREATE_TEST(test_alloc_returns_usable_memory)