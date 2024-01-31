#include <cpu.h>
#include <cpu/gdt.h>
#include <cpu/idt.h>
#include <tty/tty.h>

void arch_init(void)
{
    gdt_init();
    idt_init();
}