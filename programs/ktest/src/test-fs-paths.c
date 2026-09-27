#include "ktest.h"
#include "assert.h"

#include <nullex/fs.h>
#include <string.h>

int test_normpath_already_clean(void) {
  char out[MAX_PATH_LEN];
  char *ret = normpath("/a/b/c", out, sizeof(out));
  T_ASSERT_NOT_NULL(ret);
  T_ASSERT_STREQ(out, "/a/b/c");
  return 0;
}
CREATE_TEST(test_normpath_already_clean)

int test_normpath_collapses_redundant_slashes(void) {
  char out[MAX_PATH_LEN];
  normpath("/a//b///c", out, sizeof(out));
  T_ASSERT_STREQ(out, "/a/b/c");
  return 0;
}
CREATE_TEST(test_normpath_collapses_redundant_slashes)

int test_normpath_removes_dot_segments(void) {
  char out[MAX_PATH_LEN];
  normpath("/a/./b/./c", out, sizeof(out));
  T_ASSERT_STREQ(out, "/a/b/c");
  return 0;
}
CREATE_TEST(test_normpath_removes_dot_segments)

int test_normpath_resolves_dotdot(void) {
  char out[MAX_PATH_LEN];
  normpath("/a/b/../c", out, sizeof(out));
  T_ASSERT_STREQ(out, "/a/c");
  return 0;
}
CREATE_TEST(test_normpath_resolves_dotdot)

int test_normpath_resolves_multiple_dotdot(void) {
  char out[MAX_PATH_LEN];
  normpath("/a/b/c/../../d", out, sizeof(out));
  T_ASSERT_STREQ(out, "/a/d");
  return 0;
}
CREATE_TEST(test_normpath_resolves_multiple_dotdot)

int test_normpath_root_alone(void) {
  char out[MAX_PATH_LEN];
  normpath("/", out, sizeof(out));
  T_ASSERT_STREQ(out, "/");
  return 0;
}
CREATE_TEST(test_normpath_root_alone)

int test_normpath_null_path_returns_null(void) {
  char out[MAX_PATH_LEN];
  T_ASSERT_NULL(normpath(NULL, out, sizeof(out)));
  return 0;
}
CREATE_TEST(test_normpath_null_path_returns_null)

int test_normpath_null_out_returns_null(void) {
  T_ASSERT_NULL(normpath("/a/b", NULL, MAX_PATH_LEN));
  return 0;
}
CREATE_TEST(test_normpath_null_out_returns_null)

int test_normpath_buffer_too_small_returns_null(void) {
  char out[4];
  T_ASSERT_NULL(normpath("/a/b/c", out, sizeof(out)));
  return 0;
}
CREATE_TEST(test_normpath_buffer_too_small_returns_null)

int test_normpath_trailing_slash_is_dropped(void) {
  char out[MAX_PATH_LEN];
  normpath("/a/b/", out, sizeof(out));
  T_ASSERT_STREQ(out, "/a/b");
  return 0;
}
CREATE_TEST(test_normpath_trailing_slash_is_dropped)

int test_rslvpath_relative_joins_cwd(void) {
  char out[MAX_PATH_LEN];
  char *ret = rslvpath("foo/bar", "/home/user", out, sizeof(out));
  T_ASSERT_NOT_NULL(ret);
  T_ASSERT_STREQ(out, "/home/user/foo/bar");
  return 0;
}
CREATE_TEST(test_rslvpath_relative_joins_cwd)

int test_rslvpath_absolute_ignores_cwd(void) {
  char out[MAX_PATH_LEN];
  rslvpath("/etc/config", "/home/user", out, sizeof(out));
  T_ASSERT_STREQ(out, "/etc/config");
  return 0;
}
CREATE_TEST(test_rslvpath_absolute_ignores_cwd)

int test_rslvpath_relative_dotdot_escapes_cwd(void) {
  char out[MAX_PATH_LEN];
  rslvpath("../sibling", "/home/user/project", out, sizeof(out));
  T_ASSERT_STREQ(out, "/home/user/sibling");
  return 0;
}
CREATE_TEST(test_rslvpath_relative_dotdot_escapes_cwd)

int test_rslvpath_dot_resolves_to_cwd(void) {
  char out[MAX_PATH_LEN];
  rslvpath(".", "/home/user", out, sizeof(out));
  T_ASSERT_STREQ(out, "/home/user");
  return 0;
}
CREATE_TEST(test_rslvpath_dot_resolves_to_cwd)

int test_rslvpath_null_arguments_return_null(void) {
  char out[MAX_PATH_LEN];
  T_ASSERT_NULL(rslvpath(NULL, "/home/user", out, sizeof(out)));
  T_ASSERT_NULL(rslvpath("foo", NULL, out, sizeof(out)));
  return 0;
}
CREATE_TEST(test_rslvpath_null_arguments_return_null)

int test_joinpath_basic(void) {
  char out[MAX_PATH_LEN];
  char *ret = joinpath("/home/user", "file.txt", out, sizeof(out));
  T_ASSERT_NOT_NULL(ret);
  T_ASSERT_STREQ(out, "/home/user/file.txt");
  return 0;
}
CREATE_TEST(test_joinpath_basic)

int test_joinpath_does_not_evaluate_dotdot(void) {
  char out[MAX_PATH_LEN];
  joinpath("/a/b", "../c", out, sizeof(out));
  T_ASSERT_STREQ(out, "/a/b/../c");
  return 0;
}
CREATE_TEST(test_joinpath_does_not_evaluate_dotdot)

int test_joinpath_buffer_overflow_returns_null(void) {
  char out[8];
  T_ASSERT_NULL(joinpath("/home/user", "file.txt", out, sizeof(out)));
  return 0;
}
CREATE_TEST(test_joinpath_buffer_overflow_returns_null)

int test_joinpath_cwd_with_trailing_slash(void) {
  char out[MAX_PATH_LEN];
  joinpath("/home/user/", "file.txt", out, sizeof(out));
  T_ASSERT_STREQ(out, "/home/user/file.txt");
  return 0;
}
CREATE_TEST(test_joinpath_cwd_with_trailing_slash)