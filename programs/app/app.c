#include "../include/nullex.h"

static int is_space(char c) {
    return c == ' ' || c == '\t' || c == '\n' || c == '\r' || c == '\v' || c == '\f';
}

static void trim_newline(char *s) {
    size_t len = strlen(s);
    while (len > 0) {
        char c = s[len - 1];
        if (c != '\n' && c != '\r') {
            break;
        }
        s[len - 1] = '\0';
        len--;
    }
}

static void read_line(const char *prompt, char *buffer, size_t len) {
    if (len == 0) {
        return;
    }

    buffer[0] = '\0';
    input(prompt, buffer, len);
    trim_newline(buffer);
}

static int parse_i64(const char *s, int64_t *out) {
    size_t i = 0;
    int sign = 1;
    int64_t value = 0;

    while (s[i] && is_space(s[i])) {
        i++;
    }

    if (s[i] == '+' || s[i] == '-') {
        if (s[i] == '-') {
            sign = -1;
        }
        i++;
    }

    if (s[i] < '0' || s[i] > '9') {
        return 0;
    }

    while (s[i] >= '0' && s[i] <= '9') {
        value = value * 10 + (s[i] - '0');
        i++;
    }

    while (s[i] && is_space(s[i])) {
        i++;
    }

    if (s[i] != '\0') {
        return 0;
    }

    *out = value * sign;
    return 1;
}

static void press_enter_to_continue(void) {
    char tmp[8];
    say("\nPress Enter to continue...");
    input("", tmp, sizeof(tmp));
}

static void display_menu(void) {
    say("\n=== Nullex Terminal App ===");
    say("1. List files");
    say("2. Read file");
    say("3. Write to file");
    say("4. System info");
    say("5. Calculator");
    say("0. Exit");
    say("============================");
}

static void list_files(void) {
    say("\nList files:");
    say("This kernel/userspace interface does not expose directory enumeration yet.");
    say("So the app cannot truly list files until a directory syscall is added.");
}

static void read_file(void) {
    char filename[256];
    read_line("Enter filename: ", filename, sizeof(filename));

    if (filename[0] == '\0') {
        say("No filename entered.");
        return;
    }

    int fd = openf(filename);
    if (fd < 0) {
        say("Error: Cannot open file");
        return;
    }

    int32_t size = sizef(fd);
    if (size < 0) {
        say("Error: Cannot get file size");
        closef(fd);
        return;
    }

    if (size == 0) {
        say("(File is empty)");
        closef(fd);
        return;
    }

    /* Keep this safe and simple: cap display size */
    if (size > 1023) {
        size = 1023;
    }

    char buf[1024];
    int32_t got = readf((uint64_t)fd, (uint8_t *)buf, (size_t)size);
    if (got < 0) {
        say("Error: Failed to read file");
        closef(fd);
        return;
    }

    if (got > 1023) {
        got = 1023;
    }

    buf[got] = '\0';

    say("\n--- File content ---");
    say("%s", buf);
    say("--------------------");

    closef(fd);
}

static void write_file(void) {
    char filename[256];
    char content[512];

    read_line("Enter filename: ", filename, sizeof(filename));
    if (filename[0] == '\0') {
        say("No filename entered.");
        return;
    }

    read_line("Enter content: ", content, sizeof(content));

    int fd = openf(filename);
    if (fd < 0) {
        say("Error: Cannot open file for writing");
        say("Your current syscalls do not show a create/truncate mode, so the file must already exist.");
        return;
    }

    int32_t written = writef((uint64_t)fd, content);
    closef(fd);

    if (written < 0) {
        say("Error: Write failed");
        return;
    }

    say("File written successfully");
}

static void system_info(void) {
    say("\nSystem Information");
    say("OS: Nullex Kernel");
    say("Version: 0.1");
    say("Userspace App: Nullex Terminal App");
}

static void calculator(void) {
    char op[16];
    char a[64];
    char b[64];
    int64_t x, y;

    read_line("Operation (+, -, *, /): ", op, sizeof(op));
    read_line("Enter first number: ", a, sizeof(a));
    read_line("Enter second number: ", b, sizeof(b));

    if (!parse_i64(a, &x) || !parse_i64(b, &y)) {
        say("Invalid number input.");
        return;
    }

    if (op[0] == '+') {
        say("Result: %d", (long)(x + y));
    } else if (op[0] == '-') {
        say("Result: %d", (long)(x - y));
    } else if (op[0] == '*') {
        say("Result: %d", (long)(x * y));
    } else if (op[0] == '/') {
        if (y == 0) {
            say("Error: Division by zero");
            return;
        }
        say("Result: %d", (long)(x / y));
    } else {
        say("Unknown operation. Use +, -, *, or /.");
    }
}

int main_loop(void) {
    char choice[16];

    while (1) {
        display_menu();
        read_line("Select option: ", choice, sizeof(choice));

        if (choice[0] == '0') {
            say("Goodbye!");
            break;
        } else if (choice[0] == '1') {
            list_files();
            press_enter_to_continue();
        } else if (choice[0] == '2') {
            read_file();
            press_enter_to_continue();
        } else if (choice[0] == '3') {
            write_file();
            press_enter_to_continue();
        } else if (choice[0] == '4') {
            system_info();
            press_enter_to_continue();
        } else if (choice[0] == '5') {
            calculator();
            press_enter_to_continue();
        } else {
            say("Invalid option");
            press_enter_to_continue();
        }
    }

    return 0;
}

int main(void) {
    say("Welcome to Nullex Terminal!");
    return main_loop();
}