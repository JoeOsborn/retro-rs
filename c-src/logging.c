#include <stdio.h>
#include <stdarg.h>


void retrors_log_print(int level, const char *fmt, ...) {
    va_list va_args;
    if (level == 0) {
      printf("[DBG]: ");
    } else if(level == 1) {
      printf("[INF]: ");
    } else if(level == 2) {
      printf("[WRN]: ");
    } else if(level == 3) {
      printf("[ERR]: ");
    } else {
      printf("[%03d]: ", level);
    }
    va_start(va_args, fmt);
    vprintf(fmt, va_args);
    va_end(va_args);
}
