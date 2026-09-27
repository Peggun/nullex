#ifndef NLIBC_KTEST_ASSERT_H
#define NLIBC_KTEST_ASSERT_H
/**
 * @file ktest_assert.h
 * @brief Small assertion helpers layered on top of ktest.h.
 *
 * ktest.h's test_func_t returns int with no assertion helpers of its own,
 * so every test function here would otherwise need hand-rolled if/return
 * checks. These macros give that a consistent shape and a diagnostic
 * message. By convention a test function returns 0 on success and 1 on
 * failure, and every macro here does an early `return 1;` on failure --
 * which means they can only be used directly inside a function matching
 * test_func_t, not in a helper called from one.
 *
 * Diagnostics are printed with say() from <nullex/syscalls.h>. say() is
 * chosen deliberately: it's a static inline fully defined in the header
 * itself, so it needs nothing from the rest of nlibc to link, unlike
 * printf()/snprintf() which are extern and depend on nlibc's own stdio
 * implementation.
 */
#include <nullex/syscalls.h>
#include <stddef.h>
#include <string.h>
/** @brief Fail unless @p cond is true. */
#define T_ASSERT(cond)                                        \
  do {                                                        \
    if (!(cond)) {                                            \
      say("    FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); \
      return 1;                                                \
    }                                                          \
  } while (0)
/** @brief Fail unless the two integer expressions are equal. */
#define T_ASSERT_EQ(actual, expected)                                    \
  do {                                                                   \
    long long a_ = (long long)(actual);                                 \
    long long e_ = (long long)(expected);                               \
    if (a_ != e_) {                                                      \
      say("    FAIL %s:%d: %s == %s, got %lld, want %lld\n", __FILE__,   \
          __LINE__, #actual, #expected, a_, e_);                        \
      return 1;                                                          \
    }                                                                    \
  } while (0)
/** @brief Fail unless two pointers compare equal. */
#define T_ASSERT_PTR_EQ(actual, expected)                                \
  do {                                                                   \
    const void *a_ = (const void *)(actual);                            \
    const void *e_ = (const void *)(expected);                          \
    if (a_ != e_) {                                                      \
      say("    FAIL %s:%d: %s == %s, got %p, want %p\n", __FILE__,       \
          __LINE__, #actual, #expected, a_, e_);                        \
      return 1;                                                          \
    }                                                                    \
  } while (0)
/** @brief Fail unless both null-terminated strings are equal. Handles NULL
 *         on either side without dereferencing it. */
#define T_ASSERT_STREQ(actual, expected)                                   \
  do {                                                                     \
    const char *a_ = (actual);                                            \
    const char *e_ = (expected);                                          \
    if (a_ == NULL || e_ == NULL || strcmp(a_, e_) != 0) {                 \
      say("    FAIL %s:%d: %s == %s, got \"%s\", want \"%s\"\n", __FILE__, \
          __LINE__, #actual, #expected, a_ ? a_ : "(null)",                \
          e_ ? e_ : "(null)");                                             \
      return 1;                                                            \
    }                                                                      \
  } while (0)
/** @brief Fail unless @p len bytes at @p actual and @p expected match. */
#define T_ASSERT_MEMEQ(actual, expected, len)                             \
  do {                                                                    \
    size_t n_ = (size_t)(len);                                           \
    if (memcmp((actual), (expected), n_) != 0) {                       \
      say("    FAIL %s:%d: %s != %s over %zu bytes\n", __FILE__,         \
          __LINE__, #actual, #expected, n_);                             \
      return 1;                                                          \
    }                                                                    \
  } while (0)
/** @brief Fail unless @p ptr is NULL. */
#define T_ASSERT_NULL(ptr)                                               \
  do {                                                                   \
    if ((ptr) != NULL) {                                                 \
      say("    FAIL %s:%d: %s is not NULL\n", __FILE__, __LINE__, #ptr); \
      return 1;                                                          \
    }                                                                    \
  } while (0)
/** @brief Fail if @p ptr is NULL. */
#define T_ASSERT_NOT_NULL(ptr)                                       \
  do {                                                               \
    if ((ptr) == NULL) {                                             \
      say("    FAIL %s:%d: %s is NULL\n", __FILE__, __LINE__, #ptr); \
      return 1;                                                      \
    }                                                                \
  } while (0)
#endif