#include "ktest.h"
#include "assert.h"

#include <nullex/process.h>
#include <nullex/syscalls.h>
#include <stdint.h>

int test_spawnp_nonexistent_executable_fails(void) {
  char *argv[] = {"ktest_missing", NULL};
  int32_t pid = spawnp("/ktest_this_executable_does_not_exist", 1, argv);
  T_ASSERT(pid < 0);
  return 0;
}
CREATE_TEST(test_spawnp_nonexistent_executable_fails)

// we know that it succeeds by the test suite running and through the NUSH
// shell, so we only test the failure path currently hence the #if 0
#if 0
int test_spawnp_valid_executable_succeeds(void) {
  char *argv[] = {"/path/to/a/real/executable", NULL};
  int32_t pid = spawnp("/path/to/a/real/executable", 1, argv);
  T_ASSERT(pid >= 0);

  waiton();
  return 0;
}
CREATE_TEST(test_spawnp_valid_executable_succeeds)
#endif