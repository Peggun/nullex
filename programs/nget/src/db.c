#include "db.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "json.h"
#include "nget.h"
#include "nullex/fs.h"
#include "nullex/syscalls.h"

static char db_read_buffer[MAX_INDEX_SIZE];

void save_installed(Package *pkgs, int count) {
  // delete old file so we have a blank state
  rmfile(INSTALLED_DB);

  int fd = openf(INSTALLED_DB, O_CREAT);
  if (fd < 0) {
    printf("Failed to open %s for writing.\n", INSTALLED_DB);
    return;
  }

  char buf[512];
  int len;

  len = snprintf(buf, sizeof(buf), "[\n");
  writef(fd, (uint8_t *)buf, len);

  for (int i = 0; i < count; i++) {
    len = snprintf(
        buf, sizeof(buf),
        "  {\"name\": \"%s\", \"version\": \"%s\", \"file\": \"%s\"}%s\n",
        pkgs[i].name, pkgs[i].version, pkgs[i].local_file,
        (i == count - 1) ? "" : ",");
    writef(fd, (uint8_t *)buf, len);
  }

  len = snprintf(buf, sizeof(buf), "]\n");
  writef(fd, (uint8_t *)buf, len);

  closef(fd);
}

int load_installed(Package *pkgs, int max_pkgs) {
  int fd =
      openf(INSTALLED_DB, O_RDONLY);  // rdonly doesnt do anything currently.
  if (fd < 0) {
    return 0;
  }

  int32_t fsize = sizef(fd);
  if (fsize <= 0 || fsize >= MAX_INDEX_SIZE) {
    closef(fd);
    return 0;
  }

  readf(fd, (uint8_t *)db_read_buffer, fsize);
  db_read_buffer[fsize] = '\0';
  closef(fd);

  int count = 0;
  const char *pos = strstr(db_read_buffer, "[");
  if (!pos) return 0;
  pos++;

  while (*pos && count < max_pkgs) {
    pos = strstr(pos, "{");
    if (!pos) break;
    const char *obj_start = pos;
    int depth = 1;
    pos++;
    while (*pos && depth > 0) {
      if (*pos == '{')
        depth++;
      else if (*pos == '}')
        depth--;
      pos++;
    }

    if (depth == 0) {
      char obj_buf[1024] = {0};
      int len = pos - obj_start;
      if (len >= (int)sizeof(obj_buf)) len = sizeof(obj_buf) - 1;

      memcpy(obj_buf, obj_start, len);
      obj_buf[len] = '\0';

      Package *p = &pkgs[count];
      memset(p, 0, sizeof(Package));

      extract_json_string(obj_buf, "name", p->name, sizeof(p->name));
      extract_json_string(obj_buf, "version", p->version, sizeof(p->version));
      extract_json_string(obj_buf, "file", p->local_file,
                          sizeof(p->local_file));

      if (p->name[0] != '\0') count++;
    }
  }
  return count;
}