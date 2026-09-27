#include <stdio.h>

#include "ktest.h"

extern const test_desc_t __start_user_tests[];
extern const test_desc_t __stop_user_tests[];

void run_all_tests(void) {
  const test_desc_t *start = __start_user_tests;
  const test_desc_t *stop = __stop_user_tests;
  int num_tests = stop - start;

  int passed = 0, failed = 0;
  printf("Running %d userspace tests...\n", num_tests);

  for (int i = 0; i < num_tests; i++) {
    printf("test %d (%s)... ", i + 1, start[i].name);
    int res = start[i].func();

    if (res == 0) {
      printf("ok\n");
      passed++;
    } else {
      printf("FAILED (code %d)\n", res);
      failed++;
    }
  }
  printf("passed: %d, failed: %d\n", passed, failed);
}

int main(void) {
  run_all_tests();
  for (int i = 0; i < 1000000; i++) {}
  return 0;
}