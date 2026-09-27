#ifndef _KTEST_H
#define _KTEST_H

typedef int (*test_func_t)();

typedef struct {
  const char *name;
  test_func_t func;
} test_desc_t;

#define CREATE_TEST(func)                                                      \
  __attribute__((                                                              \
      used,                                                                    \
      section(".user_tests"))) static const test_desc_t __test_##func = { \
      #func, func};

void run_all_tests(void);

#endif