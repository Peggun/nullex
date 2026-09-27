#include "ktest.h"
#include "assert.h"

#include <nullex/sysutils.h>
#include <string.h>

int test_say_append_char_basic(void) {
  char buf[8];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_char(&p, 'x', end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "x");
  return 0;
}
CREATE_TEST(test_say_append_char_basic)

int test_say_append_char_stops_at_end(void) {
  char buf[4];
  memset(buf, 'Z', sizeof(buf));
  char *p = buf;
  char *end = buf + 2;

  say_append_char(&p, 'a', end);
  say_append_char(&p, 'b', end);
  say_append_char(&p, 'c', end);

  T_ASSERT_EQ(buf[0], 'a');
  T_ASSERT_EQ(buf[1], 'b');
  T_ASSERT_EQ(buf[2], 'Z'); 
  T_ASSERT_PTR_EQ(p, end);
  return 0;
}
CREATE_TEST(test_say_append_char_stops_at_end)

int test_say_append_str_basic(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_str(&p, "hello", end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "hello");
  return 0;
}
CREATE_TEST(test_say_append_str_basic)

int test_say_append_str_truncates_at_end(void) {
  char buf[6];
  memset(buf, 'Z', sizeof(buf));
  char *p = buf;
  char *end = buf + 4;

  say_append_str(&p, "hello world", end);

  T_ASSERT_MEMEQ(buf, "hell", 4);
  T_ASSERT_EQ(buf[4], 'Z');
  T_ASSERT_PTR_EQ(p, end);
  return 0;
}
CREATE_TEST(test_say_append_str_truncates_at_end)

int test_say_append_strn_limits_to_n(void) {
  char buf[8];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_strn(&p, "hello", 3, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "hel");
  return 0;
}
CREATE_TEST(test_say_append_strn_limits_to_n)

int test_say_append_strn_stops_at_nul_before_n(void) {
  char buf[8];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;
  
  say_append_strn(&p, "hi", 10, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "hi");
  return 0;
}
CREATE_TEST(test_say_append_strn_stops_at_nul_before_n)

int test_say_append_uint_decimal(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint(&p, 0, 10, end);
  *p = '\0';
  T_ASSERT_STREQ(buf, "0");

  p = buf;
  say_append_uint(&p, 42, 10, end);
  *p = '\0';
  T_ASSERT_STREQ(buf, "42");

  return 0;
}
CREATE_TEST(test_say_append_uint_decimal)

int test_say_append_uint_hex_is_lowercase(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint(&p, 255, 16, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "ff");
  return 0;
}
CREATE_TEST(test_say_append_uint_hex_is_lowercase)

int test_say_append_uint_binary(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint(&p, 8, 2, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "1000");
  return 0;
}
CREATE_TEST(test_say_append_uint_binary)

int test_say_append_int_sign_handling(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_int(&p, -42, 10, end);
  *p = '\0';
  T_ASSERT_STREQ(buf, "-42");

  p = buf;
  say_append_int(&p, 42, 10, end);
  *p = '\0';
  T_ASSERT_STREQ(buf, "42");

  p = buf;
  say_append_int(&p, 0, 10, end);
  *p = '\0';
  T_ASSERT_STREQ(buf, "0");

  return 0;
}
CREATE_TEST(test_say_append_int_sign_handling)

int test_say_append_uint_fmt_no_width(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 42, 10, 0, 0, 0, 0, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "42");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_no_width)

int test_say_append_uint_fmt_space_padding(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 42, 10, 5, /*zero_pad=*/0, 0, 0, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "   42");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_space_padding)

int test_say_append_uint_fmt_zero_padding(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 42, 10, 5, /*zero_pad=*/1, 0, 0, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "00042");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_zero_padding)

int test_say_append_uint_fmt_width_smaller_than_value(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 12345, 10, 2, 0, 0, 0, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "12345");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_width_smaller_than_value)

int test_say_append_uint_fmt_uppercase_hex(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 255, 16, 0, 0, 0, /*uppercase=*/1, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "FF");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_uppercase_hex)

int test_say_append_uint_fmt_zero_value_zero_padded(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 0, 10, 4, 1, 0, 0, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "0000");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_zero_value_zero_padded)

int test_say_append_uint_fmt_alt_form_hex_prefix(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 255, 16, 0, 0, /*alt_form=*/1, 0, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "0xff");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_alt_form_hex_prefix)

int test_say_append_uint_fmt_alt_form_uppercase_prefix(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 255, 16, 0, 0, 1, 1, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "0XFF");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_alt_form_uppercase_prefix)

int test_say_append_uint_fmt_alt_form_suppressed_for_zero(void) {
  // "%#x" of 0 is just "0" in the standard library
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 0, 16, 0, 0, 1, 0, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "0");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_alt_form_suppressed_for_zero)

int test_say_append_uint_fmt_alt_form_with_zero_pad_and_width(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 255, 16, 6, 1, 1, 0, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "0x00ff");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_alt_form_with_zero_pad_and_width)

int test_say_append_uint_fmt_alt_form_with_space_pad_and_width(void) {
  char buf[32];
  char *p = buf;
  char *end = buf + sizeof(buf) - 1;

  say_append_uint_fmt(&p, 255, 16, 6, 0, 1, 0, end);
  *p = '\0';

  T_ASSERT_STREQ(buf, "  0xff");
  return 0;
}
CREATE_TEST(test_say_append_uint_fmt_alt_form_with_space_pad_and_width)