#ifndef _NUSH_H
#define _NUSH_H 1

#include "../../include/stdbool.h"
#include "cat.h"
#include "cd.h"
#include "echo.h"
#include "help.h"
#include "ls.h"

#define ERR_NOT_BUILTIN_CMD -1
#define ERR_CMD_NF -2

typedef void (*BuiltinFunc)(int argc, char *argv[]);
typedef struct {
  const char *name;
  BuiltinFunc func;
  const char *help;
} FunctionMapping;

int is_in_list(const char *str, const FunctionMapping list[], int size);
void clear(int argc, char *argv[]);

static const FunctionMapping BUILTIN_CMDS[] = {
    {"clear", clear, "Clear the screen"},
    {"echo", echo, "Print arguments"},
    {"help", help, "Show available commands"},
    {"ls", ls, "Show directory contents"},
    {"cd", cd, "Change directories"},
    {"cat", cat, "See file contents"},
};

#define BUILTIN_CMDS_COUNT (sizeof(BUILTIN_CMDS) / sizeof(BUILTIN_CMDS[0]))

#endif
