#include "../include/ctype.h"

char toLower(char ch) {
    if (ch >= 'A' && ch <= 'Z') {
        return ch + 32;
    }
    return ch;
}

void strToLower(char* str) {
    int i = 0;
    while (str[i] != '\0') {
        str[i] = toLower(str[i]);
        i++;
    }
}