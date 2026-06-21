// programs/include/nullex/io.h
#ifndef _NULLEX_IO_H
#define _NULLEX_IO_H

#include "../stdint.h"
#include "../stddef.h"
#include "syscalls.h"

/*
 * input() - read a line from stdin into a caller-provided buffer.
 *
 * Usage:
 *   char buf[256];
 *   input("What is your name? ", buf, sizeof(buf));
 *   say(buf);
 */
static inline int32_t input(const char* msg, char* buffer, size_t len) {
    say("%s", msg);

    if (len == 0) {
        return 0;
    }

    int32_t bytes_read = readf(0, (uint8_t*)buffer, len - 1);
    if (bytes_read < 0) {
        bytes_read = 0;
    }

    buffer[bytes_read] = '\0';
    return bytes_read;
}

#endif