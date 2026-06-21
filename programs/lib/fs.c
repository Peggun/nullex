#include "../include/nullex/fs.h"
#include "../include/stddef.h"
#include "../include/string.h"
#include "../include/stddef.h"

char *normpath(const char *path, char *out, size_t out_size)
{
    if (!path || !out || out_size == 0) {
        return NULL;
    }

    char tmp[MAX_PATH_LEN];
    char *parts[MAX_PARTS];
    size_t count = 0;

    strncpy(tmp, path, sizeof(tmp) - 1);
    tmp[sizeof(tmp) - 1] = '\0';

    char *save = NULL;
    char *tok = strtok_r(tmp, "/", &save);

    while (tok) {
        if (strcmp(tok, ".") == 0) {
            /* skip */
        } else if (strcmp(tok, "..") == 0) {
            if (count > 0) {
                count--;
            }
        } else {
            if (count >= MAX_PARTS) {
                return NULL;
            }
            parts[count++] = tok;
        }
        tok = strtok_r(NULL, "/", &save);
    }

    if (out_size < 2) {
        return NULL;
    }

    size_t pos = 0;
    out[pos++] = '/';

    for (size_t i = 0; i < count; i++) {
        size_t len = strlen(parts[i]);

        if (pos + len + 1 >= out_size) {
            return NULL;
        }

        if (pos > 1) {
            out[pos++] = '/';
        }

        memcpy(out + pos, parts[i], len);
        pos += len;
    }

    if (pos == 1) {
        out[1] = '\0';
    } else {
        out[pos] = '\0';
    }

    return out;
}

char *rslvpath(const char *path, const char *cwd, char *out, size_t out_size) {
    if (!path || !cwd || !out || out_size == 0) {
        return NULL;
    }

    char temp[MAX_PATH_LEN];

    if (path[0] == '/') {
        strncpy(temp, path, sizeof(temp) - 1);
        temp[sizeof(temp) - 1] = '\0';
    } else {
        if (!joinpath(cwd, path, temp, sizeof(temp))) {
            return NULL;
        }
    }

    return normpath(temp, out, out_size);
}

char *joinpath(const char *cwd, const char *path, char *out, size_t out_size) {
    size_t cwd_len = strlen(cwd);
    size_t path_len = strlen(path);

    if (cwd_len + 1 + path_len + 1 > out_size)
        return NULL;

    memcpy(out, cwd, cwd_len);
    out[cwd_len] = '/';
    memcpy(out + cwd_len + 1, path, path_len);
    out[cwd_len + 1 + path_len] = '\0';

    return out;
}