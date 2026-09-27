#include "ktest.h"
#include "assert.h"

#include <nullex/dirent.h>
#include <stdint.h>

int test_opend_root_succeeds(void) {
  int32_t fd = opend("/", OPEND_NONE);
  T_ASSERT(fd >= 0);
  closed(fd);
  return 0;
}
CREATE_TEST(test_opend_root_succeeds)

int test_opend_nonexistent_path_fails(void) {
  int32_t fd = opend("/ktest_this_dir_does_not_exist", OPEND_NONE);
  T_ASSERT(fd < 0);
  return 0;
}
CREATE_TEST(test_opend_nonexistent_path_fails)

int test_opend_resolve_flag_root_succeeds(void) {
  int32_t fd = opend("/", OPEND_RESOLVE);
  T_ASSERT(fd >= 0);
  closed(fd);
  return 0;
}
CREATE_TEST(test_opend_resolve_flag_root_succeeds)

int test_getdirents_on_root_succeeds(void) {
  int32_t fd = opend("/", OPEND_NONE);
  T_ASSERT(fd >= 0);

  DirEntryInfo entries[16];
  int32_t ret = getdirents((uint64_t)fd, entries, 16);
  closed(fd);

  T_ASSERT(ret >= 0);

  if (ret > 0) {
    T_ASSERT(entries[0].kind == ENTRY_FILE ||
             entries[0].kind == ENTRY_DIRECTORY);
    T_ASSERT(entries[0].name_len > 0);
    T_ASSERT(entries[0].name[0] != '\0');
  }

  return 0;
}
CREATE_TEST(test_getdirents_on_root_succeeds)

int test_getdirents_zero_capacity(void) {
  int32_t fd = opend("/", OPEND_NONE);
  T_ASSERT(fd >= 0);

  DirEntryInfo entries[1];
  int32_t ret = getdirents((uint64_t)fd, entries, 0);
  closed(fd);

  T_ASSERT(ret >= 0);
  return 0;
}
CREATE_TEST(test_getdirents_zero_capacity)