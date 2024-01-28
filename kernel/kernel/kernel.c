#include <cpu.h>
#include <tty/tty.h>
#include <libk/io.h>

#define KERNEL_NAME "Molecule"
#define KERNEL_VER "0.0.1 - Genesis"

void kernel_main(void)
{
    tty_init();
    tty_setcolor(WHITE);
    kprintf("[ %s %s ]\n", KERNEL_NAME, KERNEL_VER);
    tty_setcolor(DEFAULT_COLOR);

    tty_colortest();

    arch_init();
    kprintf("Welcome to ");
    tty_setcolor(LIGHT_CYAN);
    kprintf("Molecule");
    tty_setcolor(DEFAULT_COLOR);
    kprintf("!\n");
}