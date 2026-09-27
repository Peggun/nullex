#include "ktest.h"
#include "assert.h"

#include <nullex/io.h>
#include <stdint.h>

int test_input_zero_length_returns_zero_without_reading(void) {
  char buf[1];
  int32_t ret = input("unused prompt", buf, 0);
  T_ASSERT_EQ(ret, 0);
  return 0;
}
CREATE_TEST(test_input_zero_length_returns_zero_without_reading)