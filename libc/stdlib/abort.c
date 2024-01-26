#include <stdio.h>
#include <stdlib.h>

__attribute__((__noreturn__))
void abort(void)
{
#ifdef __is_libk
    // TODO: Proper kernel panic
    printf("Kernel panic: abort()");
#else
    // TODO: Abort process
    printf("abort()\n");
#endif
    while(1);
    __builtin_unreachable();
}