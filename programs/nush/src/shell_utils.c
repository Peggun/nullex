#include "../include/shell_utils.h"

#include <stdio.h>

#include "../include/globals.h"

static char cwd_buffer[1024];

void update_cwd(char *path) {
  snprintf(cwd_buffer, sizeof(cwd_buffer), "%s", path);
  current_working_dir = cwd_buffer;
}