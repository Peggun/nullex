#include "../../include/nullex/io.h"
#include "../../include/nullex/fs.h"

#include "../../include/string.h"
#include "../../include/stdbool.h"
#include "../../include/stdio.h"
#include "../../include/stdlib.h"

#include "../include/nush.h"
#include "../include/globals.h"
#include "../include/shell_utils.h"

char *current_working_dir = NULL;

int main(int argc, char *argv[]) {
    printf("welcome to nush!!!\n");

    current_working_dir = "/";

    char cmd[256];

    while (1) {
        char *words[100];
        int word_count = 0;

        char *fmt_msg = format("user@nullex: %s # ", current_working_dir);
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
                printf("%s: command not found\n", words[0]);
            } else {
                char **slice = words + 1;

                BUILTIN_CMDS[list_idx].func(word_count - 1, slice);
            }
        }
    }

    return 0; // unreachable
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

void clear(int argc, char *argv[]) {
    printf("\033[H\033[2J");
}

// TODO: extend this.
void echo(int argc, char *argv[]) {
    for (int i = 0; i < argc; i++) {
        printf("%s", argv[i]);
        if (i + 1 < argc) {
            printf(" ");
        }
    }
    printf("\n");
}

void help(int argc, char *argv[]) {
    printf("Available commands: \n");
    for (int i = 0; i < BUILTIN_CMDS_COUNT; i++) {
        printf("%s - %s\n", BUILTIN_CMDS[i].name, BUILTIN_CMDS[i].help);
    }
}

void ls(int argc, char *argv[]) {
    char resolved[MAX_PATH_LEN];

    if (argc == 0) 
    {
        resolved[0] = '/';
        resolved[1] = '\0';
    }
    else
    {
        if (!rslvpath(argv[0], "/", resolved, sizeof(resolved))) {
            printf("ls: error resolving path: %s", argv[0]);
        }
    }

    DirEntryInfo entries[64];
    int fd = opend(resolved);
    int n = getdirents(fd, entries, 64);

    if (n >= 0) {
        for (int i = 0; i < n; i++) {
            if (i == n - 1) {
                printf("%.*s \n",
                    (int)entries[i].name_len,
                    entries[i].name
                );
            } else {
                printf("%.*s ",
                    (int)entries[i].name_len,
                    entries[i].name
                );
            }    
        }
    } else {
        printf("getdirents failed: %d\n", n);
    }
    return;
}
