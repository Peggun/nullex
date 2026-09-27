#ifndef NGET_DB_H
#define NGET_DB_H

#include "json.h"
#include "nget.h"

void save_installed(Package *pkgs, int count);
int load_installed(Package *pkgs, int max_pkgs);

#endif