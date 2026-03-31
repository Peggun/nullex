#include "../include/nullex.h"

int main(void) {
    for (int i = 0; i < 10; i++) {
        say("Hello!");
    }

    size_t bytes_read;
    int fd;

    fd = openf("logs/syslog");
    if (fd == -1) {
        say("error opening file");
        return -1;
    }

    size_t file_size = sizef(fd);
    char buffer[file_size];
    
    bytes_read = readf(fd, buffer, sizeof(buffer) - 1);

    const char* content = buffer;
    say(content);

    writef(fd, "Test!");

    int32_t cr = closef(fd);
    say("%d", cr);

    return 0;
}