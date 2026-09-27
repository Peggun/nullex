#include "../include/ls.h"

#include "../../include/nullex/dirent.h"
#include "../../include/nullex/fs.h"
#include "../../include/nullex/syscalls.h"
#include "../../include/stdio.h"
#include "../include/globals.h"

void ls(int argc, char *argv[]) {
  char resolved[MAX_PATH_LEN];

  if (argc == 0) {
    strncpy(resolved, current_working_dir, sizeof(resolved) - 1);
    resolved[sizeof(resolved) - 1] = '\0';
  } else {
    if (!rslvpath(argv[0], current_working_dir, resolved, sizeof(resolved))) {
      printf("ls: error resolving path: %s", argv[0]);
    }
  }

  DirEntryInfo entries[64];
  int fd = opend(resolved, 0);
  int n = getdirents(fd, entries, 64);

  if (n >= 0) {
    for (int i = 0; i < n; i++) {
      if (i == n - 1) {
        printf("%.*s \n", (int)entries[i].name_len, entries[i].name);
      } else {
        printf("%.*s ", (int)entries[i].name_len, entries[i].name);
      }
    }
  } else {
    printf("getdirents failed: %d\n", n);
  }

  closed(fd);

  return;
}
