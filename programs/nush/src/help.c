#include "../include/help.h"

#include "../../include/stdio.h"
#include "../include/nush.h"

void help(int argc, char *argv[]) {
  printf("Available commands: \n");
  for (int i = 0; i < BUILTIN_CMDS_COUNT; i++) {
    printf("%s - %s\n", BUILTIN_CMDS[i].name, BUILTIN_CMDS[i].help);
  }
}