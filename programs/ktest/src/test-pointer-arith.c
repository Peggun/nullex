#include "ktest.h"
#include "assert.h"

#include <pointer-arith.h>
#include <stdint.h>


int test_align_down_4k(void) {
  T_ASSERT_EQ(ALIGN_DOWN(4095, 4096), 0);
  T_ASSERT_EQ(ALIGN_DOWN(4096, 4096), 4096);
  T_ASSERT_EQ(ALIGN_DOWN(4097, 4096), 4096);
  return 0;
}
CREATE_TEST(test_align_down_4k)

int test_align_down_small_alignments(void) {
  T_ASSERT_EQ(ALIGN_DOWN(15, 16), 0);
  T_ASSERT_EQ(ALIGN_DOWN(16, 16), 16);
  T_ASSERT_EQ(ALIGN_DOWN(31, 16), 16);
  T_ASSERT_EQ(ALIGN_DOWN(7, 8), 0);
  T_ASSERT_EQ(ALIGN_DOWN(8, 8), 8);
  T_ASSERT_EQ(ALIGN_DOWN(9, 8), 8);
  return 0;
}
CREATE_TEST(test_align_down_small_alignments)

int test_align_down_zero_base(void) {
  T_ASSERT_EQ(ALIGN_DOWN(0, 4096), 0);
  return 0;
}
CREATE_TEST(test_align_down_zero_base)

int test_align_down_exact_multiples_are_unchanged(void) {
  for (int i = 1; i <= 8; i++) {
    int base = i * 4096;
    T_ASSERT_EQ(ALIGN_DOWN(base, 4096), base);
  }
  return 0;
}
CREATE_TEST(test_align_down_exact_multiples_are_unchanged)

int test_ptr_align_down(void) {
  void *p = (void *)(uintptr_t)0x1123; // one page + an offset
  void *aligned = PTR_ALIGN_DOWN(p, 0x1000);
  T_ASSERT_EQ((uintptr_t)aligned, 0x1000);

  // it is already aligned so it should stay the same
  void *page = (void *)(uintptr_t)0x2000;
  T_ASSERT_PTR_EQ(PTR_ALIGN_DOWN(page, 0x1000), page);

  return 0;
}
CREATE_TEST(test_ptr_align_down)

int test_ptr_align_down_preserves_pointer_type(void) {
  int buf[64];
  int *p = &buf[10];
  int *aligned = PTR_ALIGN_DOWN(p, sizeof(int) * 8);

  T_ASSERT((uintptr_t)aligned % (sizeof(int) * 8) == 0);
  T_ASSERT(aligned <= p);
  return 0;
}
CREATE_TEST(test_ptr_align_down_preserves_pointer_type)