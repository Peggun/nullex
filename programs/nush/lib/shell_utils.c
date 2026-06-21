#include "../include/shell_utils.h"
#include "../include/globals.h"

void update_cwd(const char* path) {
    current_working_dir = path;
}