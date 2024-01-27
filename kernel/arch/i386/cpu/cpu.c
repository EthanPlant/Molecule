#include <cpu.h>
#include <cpu/gdt.h>

void arch_init(void)
{
    tty_writestring("Initializing GDT ");
    gdt_init();
}