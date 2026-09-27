#include "ktest.h"
#include "assert.h"

#include <nullex/syscalls.h>
#include <stdint.h>

int test_nap_can_be_called(void) {
  int32_t ret = nap();
  (void)ret;
  return 0;
}
CREATE_TEST(test_nap_can_be_called)

int test_stop_on_implausible_pid_fails(void) {
  int32_t ret = stop((uint64_t)0xFFFFFFFFu);
  T_ASSERT(ret < 0);
  return 0;
}
CREATE_TEST(test_stop_on_implausible_pid_fails)