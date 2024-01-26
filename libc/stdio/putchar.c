#include <stdio.h>

#ifdef __is_libk
#include <kernel/tty.h>
#endif

int putchar(int ic)
{
#ifdef __is_libk
    char c = (char) ic;
    tty_putchar(c);
#else
    // TODO implement stdio
#endif

    return ic;
}