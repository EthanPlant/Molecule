#include <cpu.h>
#include <cpu/gdt.h>
#include <tty/tty.h>

void arch_init(void)
{
    gdt_init();
}