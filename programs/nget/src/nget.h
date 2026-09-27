#ifndef NGET_H
#define NGET_H

#include <nullex/net.h>
#include <nullex/syscalls.h>
#include <nullex/web/url.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define REPO_URL "https://peggun.github.io/nullex-packages"
#define INSTALLED_DB "/var/lib/nget/installed.json"
#define INSTALLED_DB_PARENT "/var/lib/nget"
#define MAX_PACKAGES 1024
#define MAX_INDEX_SIZE (2 * 1024 * 1024)  // 2MB max for the index.json

typedef struct {
  char name[128];
  char version[64];
  char description[256];
  char url[256];
  char local_file[256];
  char sha256[65];
} Package;

#endif