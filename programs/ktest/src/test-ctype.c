#include "ktest.h"
#include "assert.h"

#include <ctype.h>
#include <string.h>

int test_tolower_uppercase_letters(void) {
  T_ASSERT_EQ(toLower('A'), 'a');
  T_ASSERT_EQ(toLower('M'), 'm');
  T_ASSERT_EQ(toLower('Z'), 'z');
  return 0;
}
CREATE_TEST(test_tolower_uppercase_letters)

int test_tolower_leaves_non_uppercase_unchanged(void) {
  /* Already-lowercase letters, digits, punctuation, and whitespace should
   * all pass through untouched. */
  T_ASSERT_EQ(toLower('a'), 'a');
  T_ASSERT_EQ(toLower('z'), 'z');
  T_ASSERT_EQ(toLower('5'), '5');
  T_ASSERT_EQ(toLower('_'), '_');
  T_ASSERT_EQ(toLower(' '), ' ');
  T_ASSERT_EQ(toLower('!'), '!');
  return 0;
}
CREATE_TEST(test_tolower_leaves_non_uppercase_unchanged)

int test_tolower_boundary_characters(void) {
  T_ASSERT_EQ(toLower('@'), '@'); // one below 'A'
  T_ASSERT_EQ(toLower('['), '['); // one above 'Z'
  return 0;
}
CREATE_TEST(test_tolower_boundary_characters)

int test_strtolower_mixed_case(void) {
  char buf[] = "Nullex Kernel 0.1.0!";
  strToLower(buf);
  T_ASSERT_STREQ(buf, "nullex kernel 0.1.0!");
  return 0;
}
CREATE_TEST(test_strtolower_mixed_case)

int test_strtolower_already_lowercase_is_noop(void) {
  char buf[] = "already lower";
  strToLower(buf);
  T_ASSERT_STREQ(buf, "already lower");
  return 0;
}
CREATE_TEST(test_strtolower_already_lowercase_is_noop)

int test_strtolower_all_uppercase(void) {
  char buf[] = "SHOUTING";
  strToLower(buf);
  T_ASSERT_STREQ(buf, "shouting");
  return 0;
}
CREATE_TEST(test_strtolower_all_uppercase)

int test_strtolower_empty_string(void) {
  char buf[] = "";
  strToLower(buf);
  T_ASSERT_STREQ(buf, "");
  return 0;
}
CREATE_TEST(test_strtolower_empty_string)

int test_strtolower_preserves_length(void) {
  char buf[] = "MiXeD-Case_123";
  size_t before = strlen(buf);
  strToLower(buf);
  T_ASSERT_EQ(strlen(buf), before);
  return 0;
}
CREATE_TEST(test_strtolower_preserves_length)