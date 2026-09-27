#include <nullex/dirent.h>
#include <nullex/fs.h>
#include <nullex/io.h>
#include <nullex/process.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "../include/globals.h"
#include "../include/nush.h"
#include "../include/shell_utils.h"

char *current_working_dir = NULL;
// path where executable files should be located.
// paths are seperated by a ';' like /apps;/init;
char *PATH = "/apps;\0";

int try_path_env(int argc, char *argv[]) {
  char *cmd = argv[0];

  char path_copy[256];
  strcpy(path_copy, PATH);
  char *token = strtok(path_copy, ";");

  while (token != NULL) {
    int fd = opend(token, O_CREAT);
    if (fd < 0) {
      if (fd == -FS_ERR_INVALID_PATH) {
        printf("nush: invalid PATH path");
        continue;
      }
    }

    DirEntryInfo entries[64];
    int n = getdirents(fd, entries, 64);
    if (n < 0) {
      printf("nush: failed to get directory entries.");
      continue;
    }

    for (int i = 0; i < n; i++) {
      if (endswith(entries[i].name, ".elf") && entries[i].kind == ENTRY_FILE &&
          startswith(entries[i].name, cmd)) {
        char full_path[MAX_PATH_LEN];  // was `char *full_path[MAX_PATH_LEN]` —
                                       // array of pointers, not a char buffer
        snprintf(full_path, sizeof(full_path), "%s/%s", token, entries[i].name);

        int r = spawnp(full_path, argc, argv);

        if (r > 0) return r;
      } else {
        continue;
      }
    }

    token = strtok(NULL, ";");
  }

  return ERR_CMD_NF;
}

void print_welcome_msg() {
  printf("============================================================\n");
  printf(" /$$   /$$           /$$ /$$                    \n");
  printf("| $$$ | $$          | $$| $$                    \n");
  printf("| $$$$| $$ /$$   /$$| $$| $$  /$$$$$$  /$$   /$$\n");
  printf("| $$ $$ $$| $$  | $$| $$| $$ /$$__  $$|  $$ /$$/\n");
  printf("| $$  $$$$| $$  | $$| $$| $$| $$$$$$$$ \\  $$$$/ \n");
  printf("| $$\\  $$$| $$  | $$| $$| $$| $$_____/  >$$  $$ \n");
  printf("| $$ \\  $$|  $$$$$$/| $$| $$|  $$$$$$$ /$$/\\  $$\n");
  printf("|__/  \\__/ \\______/ |__/|__/ \\_______/|__/  \\__/\n\n");

  printf("Welcome to Nullex! v0.1.0 (built: %s)\n", __DATE__);
  printf("============================================================\n");
}

int main(int argc, char *argv[]) {
  print_welcome_msg();
  printf("Type 'help' to see available commands.\n");

  current_working_dir = "/";

  char cmd[256];

  while (1) {
    char *words[100];
    int word_count = 0;

    char fmt_msg[256];
    // format is broken for some reason.
    // TODO: fix that
    snprintf(fmt_msg, sizeof(fmt_msg), "root@nullex: %s # ",
             current_working_dir);
    input(fmt_msg, cmd, sizeof(cmd));
    char *token = strtok(cmd, " \t\n");

    while (token != NULL && word_count < 100) {
      words[word_count++] = token;
      token = strtok(NULL, " \t\n");
    }

    if (word_count > 0) {
      if (strcmp(words[0], "exit") == 0) {
        return 0;
      }

      int list_idx = is_in_list(words[0], BUILTIN_CMDS, BUILTIN_CMDS_COUNT);

      if (list_idx == ERR_NOT_BUILTIN_CMD) {
        int path_env = try_path_env(word_count, words);
        if (path_env < 0) printf("nush: command '%s' not found\n", words[0]);
      } else {
        char **slice = words + 1;
        BUILTIN_CMDS[list_idx].func(word_count - 1, slice);
      }
    }
  }

  return 0;  // unreachable
}

// returns -1 for not in.
int is_in_list(const char *str, const FunctionMapping list[], int size) {
  for (int i = 0; i < size; i++) {
    if (strcmp(list[i].name, str) == 0) {
      return i;
    }
  }
  return ERR_NOT_BUILTIN_CMD;
}

void clear(int argc, char *argv[]) { printf("\033[H\033[2J"); }
