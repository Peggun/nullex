#include "ktest.h"
#include "assert.h"

#include <nullex/fs.h>
#include <nullex/syscalls.h>
#include <stdint.h>
#include <string.h>

int test_file_write_read_roundtrip(void) {
  const char *path = "/ktest_tmp_roundtrip.txt";
  const char *content = "hello from nlibc unit tests";
  size_t content_len = strlen(content);

  int32_t fd = openf(path, O_WRONLY | O_CREAT);
  T_ASSERT(fd >= 0);

  int32_t written = writef_str((uint64_t)fd, content);
  T_ASSERT_EQ(written, (int32_t)content_len);
  closef((uint64_t)fd);

  fd = openf(path, O_RDONLY);
  T_ASSERT(fd >= 0);

  char buf[64];
  memset(buf, 0, sizeof(buf));
  int32_t read_bytes = readf((uint64_t)fd, (uint8_t *)buf, sizeof(buf) - 1);
  closef((uint64_t)fd);

  T_ASSERT_EQ(read_bytes, (int32_t)content_len);
  T_ASSERT_MEMEQ(buf, content, content_len);

  rmfile(path);
  return 0;
}
CREATE_TEST(test_file_write_read_roundtrip)

int test_file_size_reports_written_length(void) {
  const char *path = "/ktest_tmp_size.txt";
  const char *content = "0123456789";

  int32_t fd = openf(path, O_WRONLY | O_CREAT);
  T_ASSERT(fd >= 0);

  writef_str((uint64_t)fd, content);
  int32_t size = sizef((uint64_t)fd);
  closef((uint64_t)fd);

  T_ASSERT_EQ(size, 10);

  rmfile(path);
  return 0;
}
CREATE_TEST(test_file_size_reports_written_length)

int test_open_nonexistent_file_without_creat_fails(void) {
  int32_t fd = openf("/ktest_this_file_should_not_exist.txt", O_RDONLY);
  T_ASSERT(fd < 0);
  return 0;
}
CREATE_TEST(test_open_nonexistent_file_without_creat_fails)

int test_rmfile_makes_file_unopenable(void) {
  const char *path = "/ktest_tmp_rmfile.txt";

  int32_t fd = openf(path, O_WRONLY | O_CREAT);
  T_ASSERT(fd >= 0);
  writef_str((uint64_t)fd, "temporary");
  closef((uint64_t)fd);

  int32_t rm_ret = rmfile(path);
  T_ASSERT(rm_ret >= 0);

  int32_t reopened = openf(path, O_RDONLY);
  T_ASSERT(reopened < 0);
  return 0;
}
CREATE_TEST(test_rmfile_makes_file_unopenable)

int test_writef_generic_dispatches_by_argument_type(void) {
  const char *path = "/ktest_tmp_writef_macro.txt";

  int32_t fd = openf(path, O_WRONLY | O_CREAT);
  T_ASSERT(fd >= 0);

  int32_t w1 = writef((uint64_t)fd, "abc");
  T_ASSERT_EQ(w1, 3);

  uint8_t more[] = {'d', 'e', 'f'};
  int32_t w2 = writef((uint64_t)fd, more, sizeof(more));
  T_ASSERT_EQ(w2, 3);

  closef((uint64_t)fd);

  fd = openf(path, O_RDONLY);
  T_ASSERT(fd >= 0);
  char buf[16];
  memset(buf, 0, sizeof(buf));
  readf((uint64_t)fd, (uint8_t *)buf, sizeof(buf) - 1);
  closef((uint64_t)fd);

  T_ASSERT_MEMEQ(buf, "abcdef", 6);

  rmfile(path);
  return 0;
}
CREATE_TEST(test_writef_generic_dispatches_by_argument_type)

int test_rmdir_nonexistent_directory_fails(void) {
  int32_t ret = rmdir("/ktest_this_directory_should_not_exist");
  T_ASSERT(ret < 0);
  return 0;
}
CREATE_TEST(test_rmdir_nonexistent_directory_fails)

int test_compute_sha256_file_matches_known_digest(void) {
  const char *path = "/ktest_tmp_sha256.txt";
  const char *content = "Nullex filesystem test content\n";

  int32_t fd = openf(path, O_WRONLY | O_CREAT);
  T_ASSERT(fd >= 0);
  writef_str((uint64_t)fd, content);
  closef((uint64_t)fd);

  char hex[65];
  int ret = compute_sha256_file(path, hex);

  T_ASSERT_EQ(ret, 0);
  T_ASSERT_STREQ(
      hex, "f16e9ccd6930de570c2c9cb42948bb3418ce4b9e8ecc2db4ab82e29537c46e65");

  rmfile(path);
  return 0;
}
CREATE_TEST(test_compute_sha256_file_matches_known_digest)

int test_compute_sha256_file_missing_file_fails(void) {
  char hex[65];
  int ret = compute_sha256_file("/ktest_this_should_not_exist.bin", hex);
  T_ASSERT_EQ(ret, -1);
  return 0;
}
CREATE_TEST(test_compute_sha256_file_missing_file_fails)