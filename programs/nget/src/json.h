#ifndef NGET_JSON_H
#define NGET_JSON_H

#include "nget.h"

int extract_json_string(const char *json_obj, const char *key, char *out_val,
                        int max_len);
int parse_packages(const char *json, Package *pkgs, int max_pkgs);

#endif