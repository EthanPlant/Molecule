#include <stdio.h>

#include <kernel/arch.h>
#include <kernel/tty.h>

void kernel_main(void)
{
    arch_init();
    tty_init();
    printf("Hello Molecule!");
}