#include <nullex/dirent.h>
#include <nullex/fs.h>
#include <nullex/io.h>
#include <nullex/syscalls.h>
#include <nullex/web/url.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "db.h"
#include "http.h"
#include "json.h"
#include "nget.h"
#include "version.h"

#define NGET_REPOSITORY "https://peggun.github.io/nullex-packages"

/*
 * -------------------------------------------------------------------
 * Utility
 * -------------------------------------------------------------------
 */
static void print_usage(void) {
  printf(
      "Usage:\n"
      "  nget <url>                 Download a URL\n"
      "  nget search [term]         Search packages\n"
      "  nget info <package>        Show package information\n"
      "  nget install <package>     Install a package\n"
      "  nget list                  List installed packages\n"
      "  nget remove <package>      Remove a package\n"
      "  nget update                Update installed packages\n"
      "  nget --help                Show this help\n"
      "  nget --version             Show version\n");
}

static void print_version(void) { printf("nget %s\n", VERSION_STRING); }

int main(int argc, char *argv[]) {
  if (argc < 1 || argv == NULL) {
    print_usage();
    return 1;
  }

  // create the directory if not exists, if it does this just gets ignored.
  int fd = opend("/var/lib/nget", O_CREAT);
  closed(fd);

  const char *cmd = argv[0];

  if (strcmp(cmd, "--help") == 0 || strcmp(cmd, "-h") == 0) {
    print_usage();
    return 0;
  } else if (strcmp(cmd, "--version") == 0) {
    print_version();
    return 0;
  } else if (startswith(cmd, "http://") || startswith(cmd, "https://")) {
    printf("Downloading %s...\n", cmd);

    const char *filename = cmd;
    for (const char *p = cmd; *p; p++) {
      if (*p == '/') filename = p + 1;
    }

    char clean_filename[256];
    int i = 0;
    for (; filename[i] && filename[i] != '?' && i < 255; i++) {
      clean_filename[i] = filename[i];
    }
    clean_filename[i] = '\0';

    if (i == 0) {
      strcpy(clean_filename, "download.bin");
    }

    if (http_download_to_file(cmd, clean_filename) == 0) {
      printf("Saved to %s\n", clean_filename);
    } else {
      printf("Failed to download %s");
    }
    return 0;
  } else if (strcmp(cmd, "search") == 0) {
    const char *term = argc > 1 ? argv[1] : NULL;
    char *index_json = download_index(REPO_URL);
    if (!index_json) {
      printf("Failed to download index.\n");
      return 1;
    }

    static Package pkgs[MAX_PACKAGES];
    int count = parse_packages(index_json, pkgs, MAX_PACKAGES);

    printf("Available packages:\n");
    for (int i = 0; i < count; i++) {
      bool match = false;
      if (term) {
        match = (bool)(strstr(pkgs[i].name, term) != NULL ||
                       strstr(pkgs[i].description, term) != NULL);
      }
      if (match) {
        printf("  %s %-10s %s\n", pkgs[i].name, pkgs[i].version,
               pkgs[i].description);
      }
    }
    return 0;
  } else if (strcmp(cmd, "info") == 0) {
    if (argc < 2) {
      printf("Usage: nget info <package>\n");
      return 1;
    }

    const char *pkg_name = argv[1];
    char *index_json = download_index(REPO_URL);
    if (!index_json) {
      printf("Failed to download index.\n");
      return 1;
    }

    static Package pkgs[MAX_PACKAGES];
    int count = parse_packages(index_json, pkgs, MAX_PACKAGES);

    for (int i = 0; i < count; i++) {
      if (strcmp(pkgs[i].name, pkg_name) == 0) {
        printf("Name: %s\n", pkgs[i].name);
        printf("Version: %s\n", pkgs[i].version);
        printf("Description: %s\n", pkgs[i].description);
        printf("URL: %s/%s\n", REPO_URL, pkgs[i].url);
        return 0;
      }
    }
    printf("Package not found.\n");
    return 1;
  } else if (strcmp(cmd, "install") == 0) {
    if (argc < 2) {
      printf("Usage: nget install <package>\n");
      return 1;
    }
    const char *pkg_name = argv[1];

    static Package installed_check[MAX_PACKAGES];
    int inst_check_count = load_installed(installed_check, MAX_PACKAGES);
    for (int i = 0; i < inst_check_count; i++) {
      if (strcmp(installed_check[i].name, pkg_name) == 0) {
        printf(
            "Package '%s' is already installed (v%s). Use 'nget update' to "
            "upgrade if needed.\n",
            pkg_name, installed_check[i].version);
        return 0;
      }
    }

    char *index_json = download_index(REPO_URL);
    if (!index_json) {
      printf("Failed to download index.\n");
      return 1;
    }

    static Package pkgs[MAX_PACKAGES];
    int count = parse_packages(index_json, pkgs, MAX_PACKAGES);

    Package *target = NULL;
    for (int i = 0; i < count; i++) {
      if (strcmp(pkgs[i].name, pkg_name) == 0) {
        target = &pkgs[i];
        break;
      }
    }

    if (!target) {
      printf("Package not found.\n");
      return 1;
    }

    char filepath[512];
    const char *filename = target->url;
    for (const char *p = target->url; *p; p++) {
      if (*p == '/') {
        filename = p + 1;
      }
    }

    snprintf(filepath, sizeof(filepath), "/apps/%s", filename);

    char full_url[1024];
    snprintf(full_url, sizeof(full_url), "%s/%s", REPO_URL, target->url);

    printf("Downloading %s...\n", full_url);
    if (http_download_to_file(full_url, filepath) == 0) {
      if (target->sha256[0] != '\0') {
        printf("Verifying checksum...\n");
        char computed_hash[65];
        if (compute_sha256_file(filepath, computed_hash) == 0) {
          if (strcmp(computed_hash, target->sha256) != 0) {
            printf("ERROR: Checksum mismatch for %s!\n", target->name);
            printf("Expected: %s\n", target->sha256);
            printf("Got:      %s\n", computed_hash);
            printf(
                "The downloaded file may be corrupted or tampered with. "
                "Aborting installation.\n");
            rmfile(filepath);
            return 1;
          }
          printf("Checksum verified successfully.\n");
        } else {
          printf("ERROR: Failed to compute checksum for %s.\n", filepath);
          rmfile(filepath);
          return 1;
        }
      } else {
        printf("Warning: No SHA256 checksum provided for %s in index.json.\n",
               target->name);
      }

      printf("Download complete. Saving to database...\n");

      static Package installed[MAX_PACKAGES];
      int inst_count = load_installed(installed, MAX_PACKAGES);

      bool found = false;
      for (int i = 0; i < inst_count; i++) {
        if (strcmp(installed[i].name, target->name) == 0) {
          strcpy(installed[i].version, target->version);
          strcpy(installed[i].local_file, target->local_file);
          found = true;
          break;
        }
      }
      if (!found) {
        strcpy(installed[inst_count].name, target->name);
        strcpy(installed[inst_count].version, target->version);
        strcpy(installed[inst_count].local_file, target->local_file);
        inst_count++;
      }

      save_installed(installed, inst_count);
      printf("Installed %s v%s successfully.\n", target->name, target->version);
    } else {
      printf("Failed to download package.\n");
    }
    return 0;
  } else if (strcmp(cmd, "list") == 0) {
    static Package installed[MAX_PACKAGES];
    int inst_count = load_installed(installed, MAX_PACKAGES);
    if (inst_count == 0) {
      printf("No packages installed.\n");
      return 0;
    }
    printf("Installed packages:\n");
    for (int i = 0; i < inst_count; i++) {
      printf("  %-20s %-10s %s\n", installed[i].name, installed[i].version,
             installed[i].local_file);
    }
    return 0;
  } else if (strcmp(cmd, "remove") == 0) {
    if (argc < 2) {
      printf("Usage: nget remove <package_name>\n");
      return 1;
    }

    const char *pkg_name = argv[1];

    static Package installed[MAX_PACKAGES];
    int inst_count = load_installed(installed, MAX_PACKAGES);

    int found = -1;
    for (int i = 0; i < inst_count; i++) {
      if (strcmp(installed[i].name, pkg_name) == 0) {
        found = i;
        break;
      }
    }

    if (found == -1) {
      printf("Package not installed.\n");
      return 1;
    }

    char filepath[512];
    snprintf(filepath, sizeof(filepath), "installed_%s",
             installed[found].local_file);

    int32_t ret = rmfile(filepath);
    if (ret < 0) {
      printf(
          "Warning: Failed to delete file: %s. Registry will still be "
          "updated.\n",
          filepath);
    } else {
      printf("Removed %s successfully.\n", pkg_name);
    }

    for (int i = found; i < inst_count - 1; i++) {
      installed[i] = installed[i + 1];
    }
    inst_count--;

    save_installed(installed, inst_count);
    printf("Removed %s.\n", pkg_name);
    return 0;
  } else if (strcmp(cmd, "update") == 0) {
    static Package installed[MAX_PACKAGES];
    int inst_count = load_installed(installed, MAX_PACKAGES);
    if (inst_count == 0) {
      printf("No packages installed.\n");
      return 1;
    }

    char *index_json = download_index(REPO_URL);
    if (!index_json) return 1;

    static Package remote_pkgs[MAX_PACKAGES];
    int remote_count = parse_packages(index_json, remote_pkgs, MAX_PACKAGES);

    int updated = 0;
    for (int i = 0; i < inst_count; i++) {
      for (int j = 0; j < remote_count; j++) {
        if (strcmp(installed[i].name, remote_pkgs[j].name) == 0) {
          if (strcmp(installed[i].version, remote_pkgs[j].version) != 0) {
            say("Updating %s from %s to %s...\n", installed[i].name,
                installed[i].version, remote_pkgs[j].version);

            char filepath[512];
            const char *filename = remote_pkgs[j].url;
            for (const char *p = remote_pkgs[j].url; *p; p++) {
              if (*p == '/') filename = p + 1;
            }

            snprintf(filepath, sizeof(filepath), "installed_%s", filename);
            char full_url[1024];
            snprintf(full_url, sizeof(full_url), "%s/%s", REPO_URL,
                     remote_pkgs[j].url);

            if (http_download_to_file(full_url, filepath) == 0) {
              strcpy(installed[i].version, remote_pkgs[j].version);
              strcpy(installed[i].local_file, filename);
              updated++;
            }
          }
          break;
        }
      }
    }

    if (updated > 0) {
      save_installed(installed, inst_count);
      printf("Updated %d packages.\n", updated);
    } else {
      printf("All packages are up to date.\n");
    }
    return 0;
  }

  printf("Unknown command: %s\n", cmd);
  print_usage();
  return 1;
}