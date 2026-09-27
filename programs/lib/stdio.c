#include <nullex/syscalls.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
  char *buf;
  size_t cap;
  size_t len;
} FormatBuffer;

static void format_putc(FormatBuffer *out, char c) {
  if (out->buf != NULL && out->cap > 0 && out->len + 1 < out->cap) {
    out->buf[out->len] = c;
  }
  out->len++;
}

static void format_puts_n(FormatBuffer *out, const char *s, int max_len) {
  if (s == NULL) {
    s = "(null)";
  }

  for (int i = 0; s[i] != '\0'; ++i) {
    if (max_len >= 0 && i >= max_len) {
      break;
    }
    format_putc(out, s[i]);
  }
}

static void format_put_uint(FormatBuffer *out, unsigned long long num,
                            unsigned base, bool uppercase) {
  char tmp[32];
  int len = 0;

  if (num == 0) {
    tmp[len++] = '0';
  } else {
    while (num > 0) {
      unsigned digit = (unsigned)(num % base);
      if (digit < 10) {
        tmp[len++] = (char)('0' + digit);
      } else {
        tmp[len++] = (char)((uppercase ? 'A' : 'a') + digit - 10);
      }
      num /= base;
    }
  }

  for (int i = len - 1; i >= 0; --i) {
    format_putc(out, tmp[i]);
  }
}

static void format_put_int(FormatBuffer *out, long long num) {
  if (num < 0) {
    format_putc(out, '-');
    unsigned long long magnitude = (unsigned long long)(-(num + 1)) + 1;
    format_put_uint(out, magnitude, 10, false);
  } else {
    format_put_uint(out, (unsigned long long)num, 10, false);
  }
}

int vsnprintf(char *s, size_t n, const char *format, va_list args) {
    FormatBuffer out = {.buf = s, .cap = n, .len = 0};

    for (const char *f = format; *f != '\0'; ++f) {
        if (*f != '%') {
            format_putc(&out, *f);
            continue;
        }

        ++f;
        if (*f == '\0') { format_putc(&out, '%'); break; }
        if (*f == '%') { format_putc(&out, '%'); continue; }

        bool left_align = false, zero_pad = false, alt_form = false;
        bool show_sign = false, space_pad = false;
        bool parsing_flags = true;
        while (parsing_flags) {
            switch (*f) {
                case '-': left_align = true; ++f; break;
                case '0': zero_pad = true; ++f; break;
                case '#': alt_form = true; ++f; break;
                case ' ': space_pad = true; ++f; break;
                case '+': show_sign = true; ++f; break;
                default: parsing_flags = false; break;
            }
        }
        if (left_align) zero_pad = false; 

        int width = 0;
        bool has_width = false;
        if (*f == '*') {
            width = va_arg(args, int);
            if (width < 0) { left_align = true; width = -width; }
            has_width = true; ++f;
        } else {
            while (*f >= '0' && *f <= '9') {
                width = width * 10 + (*f - '0');
                has_width = true; ++f;
            }
        }
        
        int precision = -1;
        if (*f == '.') {
            ++f; precision = 0;
            if (*f == '*') {
                precision = va_arg(args, int);
                if (precision < 0) precision = -1;
                ++f;
            } else {
                while (*f >= '0' && *f <= '9') {
                    precision = precision * 10 + (*f - '0'); ++f;
                }
            }
        }

        int long_long_mod = 0, long_mod = 0, size_mod = 0;
        if (*f == 'l') { ++f; if (*f == 'l') { long_long_mod = 1; ++f; } else { long_mod = 1; } }
        else if (*f == 'z') { size_mod = 1; ++f; }
        else if (*f == 'h') { ++f; if (*f == 'h') ++f; }

        char tmp[32]; int len = 0; char sign_char = 0; const char* prefix = ""; int prefix_len = 0;

        switch (*f) {
            case 's': {
                const char *str = va_arg(args, const char *);
                if (!str) str = "(null)";
                int slen = 0; while (str[slen]) slen++;
                if (precision != -1 && slen > precision) slen = precision;
                int pad = has_width && width > slen ? width - slen : 0;
                if (!left_align) for(int i=0; i<pad; ++i) format_putc(&out, ' ');
                for(int i=0; i<slen; ++i) format_putc(&out, str[i]);
                if (left_align) for(int i=0; i<pad; ++i) format_putc(&out, ' ');
                break;
            }
            case 'c': {
                char c = (char)va_arg(args, int);
                int pad = has_width && width > 1 ? width - 1 : 0;
                if (!left_align) for(int i=0; i<pad; ++i) format_putc(&out, ' ');
                format_putc(&out, c);
                if (left_align) for(int i=0; i<pad; ++i) format_putc(&out, ' ');
                break;
            }
            case 'p': {
                uintptr_t addr = (uintptr_t)va_arg(args, void *);
                char ptmp[20]; int plen = 0;
                ptmp[plen++] = '0'; ptmp[plen++] = 'x';
                if (addr == 0) { ptmp[plen++] = '0'; } 
                else {
                    char hex[16]; int hlen = 0; uintptr_t t = addr;
                    while(t > 0) { int d = t % 16; hex[hlen++] = (d < 10) ? ('0' + d) : ('a' + d - 10); t /= 16; }
                    for(int i=hlen-1; i>=0; --i) ptmp[plen++] = hex[i];
                }
                int pad = has_width && width > plen ? width - plen : 0;
                if (!left_align) for(int i=0; i<pad; ++i) format_putc(&out, ' ');
                for(int i=0; i<plen; ++i) format_putc(&out, ptmp[i]);
                if (left_align) for(int i=0; i<pad; ++i) format_putc(&out, ' ');
                break;
            }
            case 'd': case 'i': {
                long long num = 0;
                if (long_long_mod) num = va_arg(args, long long);
                else if (long_mod) num = va_arg(args, long);
                else if (size_mod) num = va_arg(args, size_t);
                else num = va_arg(args, int);

                if (num < 0) {
                    sign_char = '-';
                    unsigned long long mag = (unsigned long long)(-(num + 1)) + 1;
                    if (mag == 0) tmp[len++] = '0';
                    while (mag > 0) { tmp[len++] = '0' + (mag % 10); mag /= 10; }
                } else {
                    if (show_sign) sign_char = '+'; else if (space_pad) sign_char = ' ';
                    unsigned long long t = (unsigned long long)num;
                    if (t == 0) tmp[len++] = '0';
                    while (t > 0) { tmp[len++] = '0' + (t % 10); t /= 10; }
                }
                for(int i=0; i<len/2; ++i) { char t=tmp[i]; tmp[i]=tmp[len-1-i]; tmp[len-1-i]=t; }
                
                int prec_pad = (precision != -1 && precision > len) ? precision - len : 0;
                int total_len = len + prec_pad + (sign_char ? 1 : 0);
                int space_pad_amt = (has_width && width > total_len) ? width - total_len : 0;
                char pad_char = (zero_pad && precision == -1 && !left_align) ? '0' : ' ';
                
                if (!left_align && pad_char == ' ') for(int i=0; i<space_pad_amt; ++i) format_putc(&out, ' ');
                if (sign_char) format_putc(&out, sign_char);
                if (!left_align && pad_char == '0') for(int i=0; i<space_pad_amt; ++i) format_putc(&out, '0');
                for(int i=0; i<prec_pad; ++i) format_putc(&out, '0');
                for(int i=0; i<len; ++i) format_putc(&out, tmp[i]);
                if (left_align) for(int i=0; i<space_pad_amt; ++i) format_putc(&out, ' ');
                break;
            }
            case 'u': case 'x': case 'X': {
                unsigned long long num = 0;
                unsigned base = (*f == 'u') ? 10 : 16;
                bool upper = (*f == 'X');
                if (long_long_mod) num = va_arg(args, unsigned long long);
                else if (long_mod) num = va_arg(args, unsigned long);
                else if (size_mod) num = va_arg(args, size_t);
                else num = va_arg(args, unsigned int);

                if (num == 0 && precision != 0) { tmp[len++] = '0'; } 
                else if (num > 0) {
                    while (num > 0) {
                        int d = num % base;
                        tmp[len++] = (d < 10) ? ('0' + d) : ((upper ? 'A' : 'a') + d - 10);
                        num /= base;
                    }
                }
                if (alt_form && base == 16 && len > 0) { prefix = upper ? "0X" : "0x"; prefix_len = 2; }
                for(int i=0; i<len/2; ++i) { char t=tmp[i]; tmp[i]=tmp[len-1-i]; tmp[len-1-i]=t; }
                
                int prec_pad = (precision != -1 && precision > len) ? precision - len : 0;
                int total_len = len + prec_pad + prefix_len;
                int space_pad_amt = (has_width && width > total_len) ? width - total_len : 0;
                char pad_char = (zero_pad && precision == -1 && !left_align) ? '0' : ' ';
                
                if (!left_align && pad_char == ' ') for(int i=0; i<space_pad_amt; ++i) format_putc(&out, ' ');
                for(int i=0; i<prefix_len; ++i) format_putc(&out, prefix[i]);
                if (!left_align && pad_char == '0') for(int i=0; i<space_pad_amt; ++i) format_putc(&out, '0');
                for(int i=0; i<prec_pad; ++i) format_putc(&out, '0');
                for(int i=0; i<len; ++i) format_putc(&out, tmp[i]);
                if (left_align) for(int i=0; i<space_pad_amt; ++i) format_putc(&out, ' ');
                break;
            }
            default: format_putc(&out, '%'); format_putc(&out, *f); break;
        }
    }

    if (s != NULL && n > 0) {
        size_t nul_index = out.len < n ? out.len : n - 1;
        s[nul_index] = '\0';
    }
    return (int)out.len;
}

int snprintf(char *s, size_t n, const char *format, ...) {
  va_list args;
  va_start(args, format);
  int ret = vsnprintf(s, n, format, args);
  va_end(args);
  return ret;
}

char *format_alloc(const char *format_string, ...) {
  va_list args;
  va_start(args, format_string);

  va_list args_copy;
  va_copy(args_copy, args);

  int needed = vsnprintf(NULL, 0, format_string, args);
  va_end(args);

  if (needed < 0) {
    va_end(args_copy);
    return NULL;
  }

  char *result = malloc((size_t)needed + 1);
  if (result == NULL) {
    va_end(args_copy);
    return NULL;
  }

  vsnprintf(result, (size_t)needed + 1, format_string, args_copy);
  va_end(args_copy);
  return result;
}

int printf(const char *restrict format, ...) {
  va_list args;
  va_start(args, format);
  int ret = vsay(format, args);
  va_end(args);
  return ret;
}
