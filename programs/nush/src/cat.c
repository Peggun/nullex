#include <errno.h>
#include <nullex/fs.h>
#include <nullex/syscalls.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "../include/globals.h"

void cat(int argc, char* argv[]) {
  if (argc < 1) {
    printf("Usage: cat <filename>\n");
    return;
  }

  char* filename = argv[0];
  char path[1024];

  if (filename[0] == '/') {
    snprintf(path, sizeof(path), "%s", filename);
  } else {
    snprintf(path, sizeof(path), "%s/%s", current_working_dir, filename);
  }

  printf("[DEBUG] Opening path: %s\n", path);

  int fd = openf(path, O_RDONLY);
  if (fd < 0) {
    if (fd == ERR_NO_ENT) {
      printf("cat: file '%s' does not exist.\n", filename);
      return;
    }
    if (fd == ERR_IS_DIR) {
      printf("cat: '%s' is a directory.\n", filename);
      return;
    }
    printf("cat: failed to open '%s' (error %d)\n", path, fd);
    return;
  }

  static char buffer[1024];
  int bytes_read;

  while ((bytes_read = readf(fd, (uint8_t*)buffer, sizeof(buffer))) > 0) {
    printf("%.*s", bytes_read, buffer);
  }

  closef(fd);
}