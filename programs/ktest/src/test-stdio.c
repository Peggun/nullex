#include "ktest.h"
#include "assert.h"

#include <stdarg.h>
#include <stdio.h>
#include <string.h>

static int call_vsnprintf(char *buf, size_t n, const char *fmt, ...) {
  va_list args;
  va_start(args, fmt);
  int ret = vsnprintf(buf, n, fmt, args);
  va_end(args);
  return ret;
}


int test_snprintf_basic_string(void) {
  char buf[32];
  int ret = snprintf(buf, sizeof(buf), "%s", "hello");
  T_ASSERT_STREQ(buf, "hello");
  T_ASSERT_EQ(ret, 5);
  return 0;
}
CREATE_TEST(test_snprintf_basic_string)

int test_snprintf_integers(void) {
  char buf[32];

  snprintf(buf, sizeof(buf), "%d", 42);
  T_ASSERT_STREQ(buf, "42");

  snprintf(buf, sizeof(buf), "%d", -7);
  T_ASSERT_STREQ(buf, "-7");

  snprintf(buf, sizeof(buf), "%d", 0);
  T_ASSERT_STREQ(buf, "0");

  return 0;
}
CREATE_TEST(test_snprintf_integers)

int test_snprintf_unsigned_and_hex(void) {
  char buf[32];

  snprintf(buf, sizeof(buf), "%u", 4294967295u);
  T_ASSERT_STREQ(buf, "4294967295");

  snprintf(buf, sizeof(buf), "%x", 255);
  T_ASSERT_STREQ(buf, "ff");

  snprintf(buf, sizeof(buf), "%X", 255);
  T_ASSERT_STREQ(buf, "FF");

  return 0;
}
CREATE_TEST(test_snprintf_unsigned_and_hex)

int test_snprintf_char_and_percent(void) {
  char buf[32];

  snprintf(buf, sizeof(buf), "%c", 'A');
  T_ASSERT_STREQ(buf, "A");

  snprintf(buf, sizeof(buf), "100%%");
  T_ASSERT_STREQ(buf, "100%");

  return 0;
}
CREATE_TEST(test_snprintf_char_and_percent)

int test_snprintf_width_zero_pad(void) {
  char buf[32];
  snprintf(buf, sizeof(buf), "%05d", 42);
  T_ASSERT_STREQ(buf, "00042");
  return 0;
}
CREATE_TEST(test_snprintf_width_zero_pad)

int test_snprintf_width_space_pad(void) {
  char buf[32];
  snprintf(buf, sizeof(buf), "%5d", 42);
  T_ASSERT_STREQ(buf, "   42");
  return 0;
}
CREATE_TEST(test_snprintf_width_space_pad)

int test_snprintf_left_align(void) {
  char buf[32];
  snprintf(buf, sizeof(buf), "%-5d|", 42);
  T_ASSERT_STREQ(buf, "42   |");
  return 0;
}
CREATE_TEST(test_snprintf_left_align)

int test_snprintf_multiple_arguments(void) {
  char buf[64];
  snprintf(buf, sizeof(buf), "%s has %d moons", "Mars", 2);
  T_ASSERT_STREQ(buf, "Mars has 2 moons");
  return 0;
}
CREATE_TEST(test_snprintf_multiple_arguments)

int test_snprintf_return_value_on_truncation(void) {
  char buf[6];
  int ret = snprintf(buf, sizeof(buf), "%s", "hello world");

  T_ASSERT_EQ(ret, 11); 
  T_ASSERT_STREQ(buf, "hello"); 
  return 0;
}
CREATE_TEST(test_snprintf_return_value_on_truncation)

int test_snprintf_truncation_is_still_nul_terminated(void) {
  char buf[4];
  snprintf(buf, sizeof(buf), "%d", 123456);
  T_ASSERT_EQ(strlen(buf), 3); 
  T_ASSERT_STREQ(buf, "123");
  return 0;
}
CREATE_TEST(test_snprintf_truncation_is_still_nul_terminated)

int test_snprintf_zero_size_writes_nothing(void) {
  char buf[4] = {'Z', 'Z', 'Z', 'Z'};
  int ret = snprintf(buf, 0, "%d", 42);

  T_ASSERT_EQ(ret, 2); 
  T_ASSERT_EQ(buf[0], 'Z');
  return 0;
}
CREATE_TEST(test_snprintf_zero_size_writes_nothing)

int test_vsnprintf_matches_snprintf(void) {
  char buf[32];
  int ret = call_vsnprintf(buf, sizeof(buf), "%s=%d", "answer", 42);

  T_ASSERT_STREQ(buf, "answer=42");
  T_ASSERT_EQ(ret, 9);
  return 0;
}
CREATE_TEST(test_vsnprintf_matches_snprintf)

int test_vsnprintf_width_and_zero_pad(void) {
  char buf[32];
  call_vsnprintf(buf, sizeof(buf), "%08x", 0xBEEF);
  T_ASSERT_STREQ(buf, "0000beef");
  return 0;
}
CREATE_TEST(test_vsnprintf_width_and_zero_pad)

int test_snprintf_string_precision_truncates(void) {
  char buf[32];
  snprintf(buf, sizeof(buf), "%.3s", "hello");
  T_ASSERT_STREQ(buf, "hel");
  return 0;
}
CREATE_TEST(test_snprintf_string_precision_truncates)