#include "json.h"

#include <string.h>

#include "nget.h"

int extract_json_string(const char *json_obj, const char *key, char *out_val,
                        int max_len) {
  char search_key[128];
  snprintf(search_key, sizeof(search_key), "\"%s\"", key);

  const char *pos = strstr(json_obj, search_key);
  if (!pos) return 0;

  pos += strlen(search_key);
  while (*pos && (*pos == ' ' || *pos == ':' || *pos == '\t' || *pos == '\n' ||
                  *pos == '\r'))
    pos++;
  if (*pos != '"') return 0;
  pos++;

  int i = 0;
  while (*pos && *pos != '"' && i < max_len - 1) {
    if (*pos == '\\' && *(pos + 1)) {
      pos++;
      if (*pos == 'n')
        out_val[i++] = '\n';
      else if (*pos == 't')
        out_val[i++] = '\t';
      else if (*pos == 'r')
        out_val[i++] = '\r';
      else if (*pos == '"')
        out_val[i++] = '"';
      else
        out_val[i++] = *pos;
    } else {
      out_val[i++] = *pos;
    }
    pos++;
  }
  out_val[i] = '\0';
  return 1;
}

int parse_packages(const char *json, Package *pkgs, int max_pkgs) {
  int count = 0;
  const char *pos = strstr(json, "[");
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
      char obj_buf[2048] = {0};
      int len = pos - obj_start;
      if (len >= (int)sizeof(obj_buf)) len = sizeof(obj_buf) - 1;

      memcpy(obj_buf, obj_start, len);
      obj_buf[len] = '\0';

      Package *p = &pkgs[count];
      memset(p, 0, sizeof(Package));
      extract_json_string(obj_buf, "name", p->name, sizeof(p->name));
      extract_json_string(obj_buf, "version", p->version, sizeof(p->version));
      extract_json_string(obj_buf, "url", p->url, sizeof(p->url));
      extract_json_string(obj_buf, "description", p->description,
                          sizeof(p->description));
      extract_json_string(obj_buf, "sha256", p->sha256, sizeof(p->sha256));

      if (p->name[0] != '\0') count++;
    }
  }
  return count;
}