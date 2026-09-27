#include "../include/cd.h"

#include "../../include/errno.h"
#include "../../include/nullex/dirent.h"
#include "../../include/nullex/fs.h"
#include "../../include/nullex/syscalls.h"
#include "../../include/stdio.h"
#include "../include/globals.h"
#include "../include/shell_utils.h"

void cd(int argc, char *argv[]) {
  char resolved[MAX_PATH_LEN];

  if (argc == 0) {
    resolved[0] = '/';
    resolved[1] = '\0';
  } else {
    if (!rslvpath(argv[0], current_working_dir, resolved, sizeof(resolved))) {
      printf("ls: error resolving path: %s", argv[0]);
    }
  }

  int fd = opend(resolved, 0);
  if (fd < 0) {
    if (fd == ERR_NOT_DIR) {
      printf("cd: '%s' is not a directory.\n", resolved);
      return;
    }
    if (fd == ERR_NO_ENT) {
      printf("cd: '%s' does not exist.\n", resolved);
      return;
    }
  }
  update_cwd(resolved);
  closed(fd);
}