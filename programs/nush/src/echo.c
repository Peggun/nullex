#include "../include/echo.h"

#include "../../include/stdio.h"

void echo(int argc, char *argv[]) {
  for (int i = 0; i < argc; i++) {
    printf("%s", argv[i]);
    if (i + 1 < argc) {
      printf(" ");
    }
  }
  printf("\n");
}