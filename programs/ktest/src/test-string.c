#include "ktest.h"
#include "assert.h"

#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

int test_strlen_basic(void) {
  T_ASSERT_EQ(strlen(""), 0);
  T_ASSERT_EQ(strlen("a"), 1);
  T_ASSERT_EQ(strlen("Nullex"), 6);
  T_ASSERT_EQ(strlen("hello world"), 11);
  return 0;
}
CREATE_TEST(test_strlen_basic)

int test_strnlen_shorter_than_maxlen(void) {
  T_ASSERT_EQ(strnlen("hi", 10), 2);
  return 0;
}
CREATE_TEST(test_strnlen_shorter_than_maxlen)

int test_strnlen_caps_at_maxlen(void) {
  T_ASSERT_EQ(strnlen("hello world", 5), 5);
  return 0;
}
CREATE_TEST(test_strnlen_caps_at_maxlen)

int test_strnlen_exact_boundary(void) {
  T_ASSERT_EQ(strnlen("hello", 5), 5);
  return 0;
}
CREATE_TEST(test_strnlen_exact_boundary)

int test_strnlen_zero_maxlen(void) {
  T_ASSERT_EQ(strnlen("anything", 0), 0);
  return 0;
}
CREATE_TEST(test_strnlen_zero_maxlen)

int test_strcmp_equal(void) {
  T_ASSERT_EQ(strcmp("abc", "abc"), 0);
  T_ASSERT_EQ(strcmp("", ""), 0);
  return 0;
}
CREATE_TEST(test_strcmp_equal)

int test_strcmp_ordering(void) {
  T_ASSERT(strcmp("abc", "abd") < 0);
  T_ASSERT(strcmp("abd", "abc") > 0);
  T_ASSERT(strcmp("ab", "abc") < 0);
  T_ASSERT(strcmp("abc", "ab") > 0);
  return 0;
}
CREATE_TEST(test_strcmp_ordering)

int test_strncmp_equal_within_n(void) {
  T_ASSERT_EQ(strncmp("abcXXX", "abcYYY", 3), 0);
  return 0;
}
CREATE_TEST(test_strncmp_equal_within_n)

int test_strncmp_differs_within_n(void) {
  T_ASSERT(strncmp("abc", "abd", 3) < 0);
  return 0;
}
CREATE_TEST(test_strncmp_differs_within_n)

int test_strncmp_zero_n_always_equal(void) {
  T_ASSERT_EQ(strncmp("completely", "different", 0), 0);
  return 0;
}
CREATE_TEST(test_strncmp_zero_n_always_equal)

int test_strncmp_one_string_shorter(void) {
  T_ASSERT(strncmp("ab", "abc", 3) < 0);
  return 0;
}
CREATE_TEST(test_strncmp_one_string_shorter)

int test_strcpy_basic(void) {
  char dest[32];
  char *ret = strcpy(dest, "hello");
  T_ASSERT_STREQ(dest, "hello");
  T_ASSERT_PTR_EQ(ret, dest);
  return 0;
}
CREATE_TEST(test_strcpy_basic)

int test_strcpy_empty_source(void) {
  char dest[8] = "xxxxxxx";
  strcpy(dest, "");
  T_ASSERT_STREQ(dest, "");
  return 0;
}
CREATE_TEST(test_strcpy_empty_source)

int test_strncpy_pads_when_source_shorter(void) {
  char dest[8];
  memset(dest, 'X', sizeof(dest));
  strncpy(dest, "hi", 8);
  // "hi" + 6 padding NULs.
  T_ASSERT_MEMEQ(dest, "hi\0\0\0\0\0\0", 8);
  return 0;
}
CREATE_TEST(test_strncpy_pads_when_source_shorter)

int test_strncpy_no_overrun_when_source_longer(void) {
  char dest[9];
  dest[5] = (char)0x7E; // sentinel one past the 5 bytes we ask to be copied
  strncpy(dest, "abcdefgh", 5);
  T_ASSERT_MEMEQ(dest, "abcde", 5);
  T_ASSERT_EQ(dest[5], (char)0x7E); // untouched, strncpy must not overrun n
  return 0;
}
CREATE_TEST(test_strncpy_no_overrun_when_source_longer)

int test_strcat_basic(void) {
  char dest[32] = "foo";
  char *ret = strcat(dest, "bar");
  T_ASSERT_STREQ(dest, "foobar");
  T_ASSERT_PTR_EQ(ret, dest);
  return 0;
}
CREATE_TEST(test_strcat_basic)

int test_strcat_empty_suffix_is_noop(void) {
  char dest[16] = "unchanged";
  strcat(dest, "");
  T_ASSERT_STREQ(dest, "unchanged");
  return 0;
}
CREATE_TEST(test_strcat_empty_suffix_is_noop)

int test_strcat_onto_empty_dest(void) {
  char dest[16] = "";
  strcat(dest, "grown");
  T_ASSERT_STREQ(dest, "grown");
  return 0;
}
CREATE_TEST(test_strcat_onto_empty_dest)

int test_strchr_found(void) {
  const char *s = "hello world";
  T_ASSERT_PTR_EQ(strchr(s, 'h'), s);       
  T_ASSERT_PTR_EQ(strchr(s, 'w'), s + 6);   
  T_ASSERT_PTR_EQ(strchr(s, 'd'), s + 10);  
  return 0;
}
CREATE_TEST(test_strchr_found)

int test_strchr_not_found(void) {
  T_ASSERT_NULL(strchr("hello", 'z'));
  return 0;
}
CREATE_TEST(test_strchr_not_found)

int test_strchr_nul_terminator(void) {
  const char *s = "abc";
  T_ASSERT_PTR_EQ(strchr(s, '\0'), s + 3);
  return 0;
}
CREATE_TEST(test_strchr_nul_terminator)

int test_strrchr_returns_last_match(void) {
  const char *s = "abcabcabc";
  T_ASSERT_PTR_EQ(strrchr(s, 'a'), s + 6);
  return 0;
}
CREATE_TEST(test_strrchr_returns_last_match)

int test_strrchr_single_match(void) {
  const char *s = "needle haystack";
  T_ASSERT_PTR_EQ(strrchr(s, 'n'), s);
  return 0;
}
CREATE_TEST(test_strrchr_single_match)

int test_strrchr_not_found(void) {
  T_ASSERT_NULL(strrchr("hello", 'z'));
  return 0;
}
CREATE_TEST(test_strrchr_not_found)

int test_strstr_found(void) {
  const char *s = "the quick brown fox";
  T_ASSERT_PTR_EQ(strstr(s, "quick"), s + 4);
  T_ASSERT_PTR_EQ(strstr(s, "the"), s);
  T_ASSERT_PTR_EQ(strstr(s, "fox"), s + 16);
  return 0;
}
CREATE_TEST(test_strstr_found)

int test_strstr_whole_string_match(void) {
  const char *s = "exact";
  T_ASSERT_PTR_EQ(strstr(s, "exact"), s);
  return 0;
}
CREATE_TEST(test_strstr_whole_string_match)

int test_strstr_empty_needle_returns_haystack(void) {
  const char *s = "haystack";
  T_ASSERT_PTR_EQ(strstr(s, ""), s);
  return 0;
}
CREATE_TEST(test_strstr_empty_needle_returns_haystack)

int test_strstr_not_found(void) {
  T_ASSERT_NULL(strstr("the quick brown fox", "cat"));
  return 0;
}
CREATE_TEST(test_strstr_not_found)

int test_strspn_initial_segment(void) {
  T_ASSERT_EQ(strspn("12345abc", "0123456789"), 5);
  return 0;
}
CREATE_TEST(test_strspn_initial_segment)

int test_strspn_no_match_at_start(void) {
  T_ASSERT_EQ(strspn("abc123", "0123456789"), 0);
  return 0;
}
CREATE_TEST(test_strspn_no_match_at_start)

int test_strspn_entire_string_matches(void) {
  T_ASSERT_EQ(strspn("aaaa", "a"), 4);
  return 0;
}
CREATE_TEST(test_strspn_entire_string_matches)

int test_strcspn_initial_segment(void) {
  T_ASSERT_EQ(strcspn("hello,world", ","), 5);
  return 0;
}
CREATE_TEST(test_strcspn_initial_segment)

int test_strcspn_first_char_rejected(void) {
  T_ASSERT_EQ(strcspn(",hello", ","), 0);
  return 0;
}
CREATE_TEST(test_strcspn_first_char_rejected)

int test_strcspn_no_reject_chars_present(void) {
  T_ASSERT_EQ(strcspn("hello", "xyz"), strlen("hello"));
  return 0;
}
CREATE_TEST(test_strcspn_no_reject_chars_present)

int test_strpbrk_finds_first_match(void) {
  const char *s = "hello world";
  T_ASSERT_PTR_EQ(strpbrk(s, "wor"), s + 4);
  return 0;
}
CREATE_TEST(test_strpbrk_finds_first_match)

int test_strpbrk_not_found(void) {
  T_ASSERT_NULL(strpbrk("hello", "xyz"));
  return 0;
}
CREATE_TEST(test_strpbrk_not_found)

int test_strtok_full_sequence(void) {
  char buf[] = "  one,two,,three  ";
  char *tok;

  tok = strtok(buf, " ,");
  T_ASSERT_NOT_NULL(tok);
  T_ASSERT_STREQ(tok, "one");

  tok = strtok(NULL, " ,");
  T_ASSERT_NOT_NULL(tok);
  T_ASSERT_STREQ(tok, "two");

  tok = strtok(NULL, " ,");
  T_ASSERT_NOT_NULL(tok);
  T_ASSERT_STREQ(tok, "three");

  tok = strtok(NULL, " ,");
  T_ASSERT_NULL(tok);

  return 0;
}
CREATE_TEST(test_strtok_full_sequence)

int test_strtok_single_token_no_delimiters(void) {
  char buf[] = "onlyoneword";
  char *tok = strtok(buf, " ,");
  T_ASSERT_STREQ(tok, "onlyoneword");
  T_ASSERT_NULL(strtok(NULL, " ,"));
  return 0;
}
CREATE_TEST(test_strtok_single_token_no_delimiters)

int test_strtok_r_basic_sequence(void) {
  char buf[] = "a:b:c";
  char *save = NULL;

  char *tok = strtok_r(buf, ":", &save);
  T_ASSERT_STREQ(tok, "a");
  tok = strtok_r(NULL, ":", &save);
  T_ASSERT_STREQ(tok, "b");
  tok = strtok_r(NULL, ":", &save);
  T_ASSERT_STREQ(tok, "c");
  tok = strtok_r(NULL, ":", &save);
  T_ASSERT_NULL(tok);

  return 0;
}
CREATE_TEST(test_strtok_r_basic_sequence)

int test_strtok_r_independent_sequences_interleaved(void) {
  // the whole point of the _r variant is that two tokenizations can be driven
  // side by side without one corrupting the others position
  char buf1[] = "1-2-3";
  char buf2[] = "x/y/z";
  char *save1 = NULL, *save2 = NULL;

  char *t1 = strtok_r(buf1, "-", &save1);
  char *t2 = strtok_r(buf2, "/", &save2);
  T_ASSERT_STREQ(t1, "1");
  T_ASSERT_STREQ(t2, "x");

  t1 = strtok_r(NULL, "-", &save1);
  t2 = strtok_r(NULL, "/", &save2);
  T_ASSERT_STREQ(t1, "2");
  T_ASSERT_STREQ(t2, "y");

  t1 = strtok_r(NULL, "-", &save1);
  t2 = strtok_r(NULL, "/", &save2);
  T_ASSERT_STREQ(t1, "3");
  T_ASSERT_STREQ(t2, "z");

  return 0;
}
CREATE_TEST(test_strtok_r_independent_sequences_interleaved)

/* ------------------------------------------------------------------ */
/* strdup                                                               */
/*                                                                       */
/* strdup needs a working allocator underneath, so this only proves the */
/* call shape compiles here; it needs the real allocator on target to   */
/* actually run.                                                        */
/* ------------------------------------------------------------------ */

int test_strdup_copies_content(void) {
  const char *original = "duplicate me";
  char *copy = strdup(original);

  T_ASSERT_NOT_NULL(copy);
  T_ASSERT_STREQ(copy, original);
  T_ASSERT(copy != original); /* must be a distinct allocation */

  free(copy);
  return 0;
}
CREATE_TEST(test_strdup_copies_content)

int test_memcpy_basic(void) {
  char dest[16];
  memset(dest, 0, sizeof(dest));
  memcpy(dest, "hello", 5);
  T_ASSERT_MEMEQ(dest, "hello", 5);
  return 0;
}
CREATE_TEST(test_memcpy_basic)

int test_memcpy_is_binary_safe(void) {
  const unsigned char src[] = {0x01, 0x00, 0xFF, 0x00, 0x42};
  unsigned char dest[5] = {0};
  memcpy(dest, src, sizeof(src));
  T_ASSERT_MEMEQ(dest, src, sizeof(src));
  return 0;
}
CREATE_TEST(test_memcpy_is_binary_safe)

int test_memcpy_returns_dest(void) {
  char dest[8];
  void *ret = memcpy(dest, "abcdefg", 7);
  T_ASSERT_PTR_EQ(ret, dest);
  return 0;
}
CREATE_TEST(test_memcpy_returns_dest)

int test_memset_fills_buffer(void) {
  unsigned char buf[10];
  memset(buf, 0xAB, sizeof(buf));
  for (size_t i = 0; i < sizeof(buf); i++) {
    T_ASSERT_EQ(buf[i], (unsigned char)0xAB);
  }
  return 0;
}
CREATE_TEST(test_memset_fills_buffer)

int test_memset_zero_length_is_noop(void) {
  unsigned char buf[4] = {1, 2, 3, 4};
  memset(buf, 0, 0);
  unsigned char expected[4] = {1, 2, 3, 4};
  T_ASSERT_MEMEQ(buf, expected, 4);
  return 0;
}
CREATE_TEST(test_memset_zero_length_is_noop)

int test_memset_only_uses_low_byte_of_value(void) {
  unsigned char buf[4];
  memset(buf, 0x1FF /* low byte is 0xFF */, sizeof(buf));
  for (size_t i = 0; i < sizeof(buf); i++) {
    T_ASSERT_EQ(buf[i], (unsigned char)0xFF);
  }
  return 0;
}
CREATE_TEST(test_memset_only_uses_low_byte_of_value)

int test_memchr_found(void) {
  const char *s = "hello world";
  T_ASSERT_PTR_EQ(memchr(s, 'w', strlen(s)), s + 6);
  return 0;
}
CREATE_TEST(test_memchr_found)

int test_memchr_not_found(void) {
  T_ASSERT_NULL(memchr("hello", 'z', 5));
  return 0;
}
CREATE_TEST(test_memchr_not_found)

int test_memchr_finds_byte_past_embedded_nul(void) {
  const unsigned char data[] = {'a', 'b', 0x00, 'c', 'd'};
  T_ASSERT_PTR_EQ(memchr(data, 'd', sizeof(data)), &data[4]);
  return 0;
}
CREATE_TEST(test_memchr_finds_byte_past_embedded_nul)

int test_memchr_respects_length_limit(void) {
  const char *s = "hello world";
  T_ASSERT_NULL(memchr(s, 'w', 5));
  return 0;
}
CREATE_TEST(test_memchr_respects_length_limit)

int test_startswith_true_and_false(void) {
  T_ASSERT(startswith("hello world", "hello") == true);
  T_ASSERT(startswith("hello world", "world") == false);
  return 0;
}
CREATE_TEST(test_startswith_true_and_false)

int test_startswith_full_string_prefix(void) {
  T_ASSERT(startswith("exact", "exact") == true);
  return 0;
}
CREATE_TEST(test_startswith_full_string_prefix)

int test_startswith_prefix_longer_than_string(void) {
  T_ASSERT(startswith("hi", "hello") == false);
  return 0;
}
CREATE_TEST(test_startswith_prefix_longer_than_string)

int test_startswith_empty_prefix(void) {
  T_ASSERT(startswith("anything", "") == true);
  return 0;
}
CREATE_TEST(test_startswith_empty_prefix)

int test_endswith_true_and_false(void) {
  T_ASSERT(endswith("hello world", "world") == true);
  T_ASSERT(endswith("hello world", "hello") == false);
  return 0;
}
CREATE_TEST(test_endswith_true_and_false)

int test_endswith_full_string_suffix(void) {
  T_ASSERT(endswith("exact", "exact") == true);
  return 0;
}
CREATE_TEST(test_endswith_full_string_suffix)

int test_endswith_suffix_longer_than_string(void) {
  T_ASSERT(endswith("hi", "hello") == false);
  return 0;
}
CREATE_TEST(test_endswith_suffix_longer_than_string)

int test_endswith_empty_suffix(void) {
  T_ASSERT(endswith("anything", "") == true);
  return 0;
}
CREATE_TEST(test_endswith_empty_suffix)

int test_endswith_common_use_case(void) {
  T_ASSERT(endswith("archive.tar.gz", ".gz") == true);
  T_ASSERT(endswith("archive.tar.gz", ".zip") == false);
  return 0;
}
CREATE_TEST(test_endswith_common_use_case)