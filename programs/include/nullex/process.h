#ifndef NULLEX_PROCESS_H
#define NULLEX_PROCESS_H

#include <nullex/syscalls.h>
#include <stddef.h>
#include <string.h>

/**
 * @brief Spawn a new process from an executable path.
 *
 * @param[in] path Path to the executable.
 * @param[in] argc Number of command-line arguments.
 * @param[in] argv Array of command-line argument strings.
 * @return Process identifier on success, otherwise a negative error code.
 */
static inline int32_t spawnp(const char *path, int argc, char *argv[]) {
  size_t len = strlen(path);

  return ksyscall(SYS_SPAWNP, (uint64_t)path, (uint64_t)len, (uint64_t)argc,
                  (uint64_t)argv, 0, 0);
}

#endif