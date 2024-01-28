#ifndef KERNEL_TTY_H
#define KERNEL_TTY_H

#include <stddef.h>

typedef enum color_t
{
    BLACK = 0,
    BLUE = 1,
    GREEN = 2,
    CYAN = 3,
    RED = 4,
    MAGENTA = 5,
    BROWN = 6,
    LIGHT_GREY = 7,
    DARK_GREY = 8,
    LIGHT_BLUE = 9,
    LIGHT_GREEN = 10,
    LIGHT_CYAN = 11,
    LIGHT_RED = 12,
    LIGHT_MAGENTA = 13,
    LIGHT_BROWN = 14,
    WHITE = 15,
} color_t;

#define DEFAULT_COLOR LIGHT_GREY

void tty_init(void);
void tty_write(const char *data, size_t len);
void tty_writestring(const char *str);
void tty_setcolor(color_t color);
void tty_colortest(void);

#endif